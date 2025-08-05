use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use reqwest::Client;
use serde::Deserialize;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

use crate::{
    API_BASE, Result,
    event::Event,
    post::Post,
    range::Range,
    task::{Task, TaskID},
};

pub struct Engine {
    tasks: VecDeque<Task>,
    failures: VecDeque<Task>,
    jobs: HashMap<TaskID, Task>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            jobs: HashMap::new(),
            failures: VecDeque::new(),
        }
    }

    pub async fn start(
        mut self,
        platform: &'static str,
        uid: u64,
        range: Range,
        cover: bool,
        out: &'static str,
        template: &'static str,
        jobs: usize,
    ) -> Result<Receiver<Event>> {
        // chann for UI interactions
        let (ui_tx, ui_rx) = mpsc::channel(128);
        // chann for downloader
        let (tx, mut rx) = mpsc::channel(128);
        // get username
        #[derive(Deserialize)]
        struct Profile {
            name: String,
            #[allow(unused)]
            public_id: String,
        }
        let client = Client::new();
        let profile = client
            .get(format!("{API_BASE}/{platform}/user/{uid}/profile"))
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?
            .json::<Profile>()
            .await?;
        let username = profile.name.leak();
        tokio::spawn(async move {
            // collect posts
            let posts = match Post::collect(platform, uid, range).await {
                Ok(posts) => posts,
                Err(err) => {
                    ui_tx.send(Event::NoPosts(err)).await.unwrap();
                    return;
                }
            };
            ui_tx.send(Event::Posts(posts.len())).await.unwrap();

            // convert them into (pending) tasks
            self.tasks = match Task::prep(posts, uid, username, cover, &out, template).await {
                Ok(tasks) => tasks,
                Err(err) => {
                    ui_tx.send(Event::NoTasks(err)).await.unwrap();
                    return;
                }
            };
            ui_tx.send(Event::Tasks(self.tasks.len())).await.unwrap();

            // spawning jobs according to the set parallelism
            let mut set = JoinSet::new();
            while self.jobs.len() < jobs {
                if self.run_more(tx.clone(), &mut set) {
                    continue;
                }
                break;
            }
            // since there's a `tx` above for cloning, `while let` never breaks.
            // thus the number of jobs running becomes the termination condition
            // jobs will run until Event::Finished is processed by the loop below
            while !set.is_empty() {
                tokio::select! {
                    // mandatory. the finished jobs won't leave the set if not joined
                    _ = set.join_next() => (),
                    // the real loop body
                    Some(event) = rx.recv() => {
                        match &event {
                            Event::NoPosts(..) => unreachable!(),
                            Event::Posts(..) => unreachable!(),
                            Event::NoTasks(..) => unreachable!(),
                            Event::Tasks(..) => unreachable!(),
                            Event::Enqueue(..) => (),
                            Event::Start(..) => (),
                            Event::Updated(..) => (),
                            Event::Fail(id, _err) => {
                                // save failed tasks for later retry (not yet implemented)
                                let task = self.jobs.remove(id).unwrap();
                                self.failures.push_back(task);
                            }
                            Event::Finished(id) => {
                                self.jobs.remove(id);
                            }
                        };
                        if matches!(&event, Event::Fail(..) | Event::Finished(..)) {
                            self.run_more(tx.clone(), &mut set);
                            println!("size of set {}", set.len());
                        }
                        ui_tx.send(event).await.unwrap();
                    }
                }
            }
            println!("out of loop");
            assert_eq!(tx.strong_count(), 1);
        });
        Ok(ui_rx)
    }

    fn run_more(&mut self, tx: Sender<Event>, set: &mut JoinSet<()>) -> bool {
        match self.tasks.pop_front() {
            Some(task) => {
                set.spawn(task.clone().start(tx));
                self.jobs.insert(task.id(), task);
                true
            }
            None => false,
        }
    }
}
