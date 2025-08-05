use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

use crate::{
    Result,
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
        user_id: u64,
        range: Range,
        cover: bool,
        out: &'static str,
        template: &'static str,
        jobs: usize,
    ) -> Result<Receiver<Event>> {
        // chann for UI
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            // get username
            let username = match Post::profile(platform, user_id).await {
                Ok(username) => username,
                Err(e) => {
                    tx.send(Event::NoProfile(e)).await.unwrap();
                    return;
                }
            };
            // scrape posts
            let posts = match Post::scrape(platform, user_id, range, tx.clone()).await {
                Ok(posts) => posts,
                Err(err) => {
                    tx.send(Event::NoPosts(err)).await.unwrap();
                    return;
                }
            };
            // convert posts into (pending) tasks
            self.tasks = match Task::create(
                posts,
                user_id,
                username,
                cover,
                &out,
                template,
                tx.clone(),
            )
            .await
            {
                Ok(tasks) => tasks,
                Err(err) => {
                    tx.send(Event::NoTasks(err)).await.unwrap();
                    return;
                }
            };
            // download
            self.download(jobs, tx).await;
        });
        Ok(rx)
    }

    async fn download(mut self, jobs: usize, ui_tx: Sender<Event>) {
        let (tx, mut rx) = mpsc::channel(128);
        // spawning jobs according to the set parallelism
        let mut set = JoinSet::new();
        while self.jobs.len() < jobs {
            if self.run_more(tx.clone(), &mut set).await {
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
                        Event::Enqueue(..) => (),
                        Event::Established(..) => (),
                        Event::Updated(..) => (),
                        Event::Failed(id, _err) => {
                            // save failed tasks for later retry (not yet implemented)
                            let task = self.jobs.remove(id).unwrap();
                            self.failures.push_back(task);
                        }
                        Event::Finished(id) => {
                            self.jobs.remove(id);
                        }
                        _ => unreachable!()
                    };
                    if matches!(&event, Event::Failed(..) | Event::Finished(..)) {
                        self.run_more(tx.clone(), &mut set).await;
                        println!("size of set {}", set.len());
                    }
                    ui_tx.send(event).await.unwrap();
                }
            }
        }
        assert_eq!(tx.strong_count(), 1);
    }

    async fn run_more(&mut self, tx: Sender<Event>, set: &mut JoinSet<()>) -> bool {
        match self.tasks.pop_front() {
            Some(task) => {
                tx.send(Event::Enqueue(task.clone())).await.unwrap();
                set.spawn(task.clone().start(tx));
                self.jobs.insert(task.id(), task);
                true
            }
            None => false,
        }
    }
}
