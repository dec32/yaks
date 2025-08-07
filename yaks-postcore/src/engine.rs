use std::ops::RangeInclusive;
use async_channel::{self, Receiver, Sender};

use crate::{
    job, post::{self, PostID}, worker::{self, Prog}, Event, Job, JobID, UserID
};

pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(
        self,
        platform: &'static str,
        user_id: UserID,
        range: RangeInclusive<PostID>,
        cover: bool,
        out: &'static str,
        template: &'static str,
        workers: u8,
    ) -> Receiver<crate::Result<Event>> {
        // event chann (for TUI/GUI)
        let (events, event_rx) = async_channel::unbounded();
        // chann for Error
        let (error_tx, errors) = async_channel::unbounded();
        listen_errors(errors, events.clone());

        tokio::spawn(async move {
            // fetching profile
            let profile = match post::fetch_profile(platform, user_id).await {
                Ok(profile) => profile,
                Err(e) => {
                    println!("Error fetching profile {e}");
                    error_tx.send(crate::Error::Profile(e)).await.unwrap();
                    return;
                }
            };
            events.send(Ok(Event::Profile(profile))).await.unwrap();
            // scrape all posts
            let posts = match post::scrape_posts(platform, user_id, range).await {
                Ok(posts) => {
                    events.send(Ok(Event::Posts(posts.len()))).await.unwrap();
                    posts
                }
                Err(e) => {
                    println!("Error creating posts {e}");
                    error_tx.send(crate::Error::Scrape(e)).await.unwrap();
                    return;
                }
            };
            events.send(Ok(Event::PostsExhausted)).await.unwrap();
            // create jobs. each job will have two copies. one for download and one for UI.
            let jobs_rx = job::create_jobs(
                posts,
                platform,
                user_id,
                profile,
                cover,
                out,
                template,
                error_tx.clone(),
            );
            let jobs = listen_jobs(jobs_rx, events.clone());
            // download
            let progress = worker::start_workers(workers, jobs.clone(), error_tx);
            listen_prog(progress, events);
        });
        event_rx
    }
}


fn listen_errors(errors: Receiver<crate::Error>, events: Sender<crate::Result<Event>>) {
    tokio::spawn(async move {
        while let Ok(e) = errors.recv().await {
            events.send(Err(e)).await.unwrap();
        }
    });
}

fn listen_jobs(jobs_rx: Receiver<Vec<Job>>, events: Sender<crate::Result<Event>>) -> Receiver<Job> {
    let (tx, rx) = async_channel::unbounded();
    tokio::spawn(async move {
        while let Ok(jobs) = jobs_rx.recv().await {
            for job in jobs.iter().cloned() {
                tx.send(job).await.unwrap();
            }
            events.send(Ok(Event::Jobs(jobs))).await.unwrap();
        }
        events.send(Ok(Event::JobExhausted)).await.unwrap();
    });
    rx
}

fn listen_prog(prog: Receiver<(JobID, Prog)>, events: Sender<crate::Result<Event>>) {
    tokio::spawn(async move {
        while let Ok((id, prog)) = prog.recv().await {
            let event = match prog {
                Prog::Enque => Event::Enqueue(id),
                Prog::Init(size) => Event::Init(id, size),
                Prog::Chunk(size) => Event::Chunk(id, size),
                Prog::Fin => Event::Fin(id),
            };
            events.send(Ok(event)).await.unwrap();
        }
        events.send(Ok(Event::Clear)).await.unwrap();
    });
}