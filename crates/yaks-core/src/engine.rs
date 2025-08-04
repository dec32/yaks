use std::collections::{HashMap, VecDeque};

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

use crate::{
    Result,
    event::Event,
    post::{Post, Range},
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
        // chann for UI interactions
        let (ui_tx, ui_rx) = mpsc::channel(128);
        // chann for downloader
        let (tx, mut rx) = mpsc::channel(128);

        // collecting posts and convert them into (pending) tasks
        let posts = Post::collect(platform, user_id, range).await?;
        self.tasks = Task::prep(posts, cover, &out, template).await?;

        if self.tasks.is_empty() {
            return Ok(ui_rx);
        }

        tokio::spawn(async move {
            let mut set = JoinSet::new();
            let tx = tx;
            // spawning jobs according to the set parallelism
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
                if let Some(event) = rx.recv().await {
                    match &event {
                        Event::Prep(..) | Event::Started(..) | Event::Updated(..) => (),
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
                    }
                    ui_tx.send(event).await.expect("UI receiver is closed.")
                }
            }
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
