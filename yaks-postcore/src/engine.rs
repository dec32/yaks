use std::ops::RangeInclusive;

use futures::FutureExt;
use smol::channel::{self, Receiver};

use crate::{
    Event, job, post,
    worker::{self, Progress},
};

pub struct Engine {}

impl Engine {
    pub fn start(
        platform: &'static str,
        user_id: u64,
        range: RangeInclusive<u64>,
        workers: u8,
    ) -> Receiver<crate::Result<Event>> {
        use Event as E;
        use Progress as P;

        // event chann (for TUI/GUI)
        let (events, event_rx) = channel::unbounded();

        // chann for Error
        let (error_tx, errors) = channel::unbounded();

        smol::spawn(async move {
            let profile = match post::fetch_profile(platform, user_id).await {
                Ok(profile) => profile,
                Err(e) => {
                    error_tx.send(crate::Error::Profile(e)).await.unwrap();
                    return;
                }
            };
            let posts = match post::scrape_posts(platform, user_id, profile, range).await {
                Ok(posts) => posts,
                Err(e) => {
                    error_tx.send(crate::Error::Scrape(e)).await.unwrap();
                    return;
                }
            };
            let jobs = job::create_jobs(posts, error_tx.clone());
            let progress = worker::start_workers(workers, jobs.clone(), error_tx);
            // event bus
            futures::select! {
                next = jobs.recv().fuse() => {
                    let event = match next {
                        Ok(job) => E::Job(job),
                        Err(_) => E::JobExhausted
                    };
                    events.send(Ok(event)).await.unwrap();
                },
                next = progress.recv().fuse() => {
                    let event = match next {
                        Ok((id, P::Init(total))) => Ok(E::Init(id, total)),
                        Ok((id, P::Chunk(bytes))) => Ok(E::Chunk(id, bytes)),
                        Ok((id, P::Fin)) => Ok(E::Fin(id)),
                        Err(_) => Ok(E::Clear),
                    };
                    events.send(event).await.unwrap();
                },
                next = errors.recv().fuse() => {
                    if let Ok(e) = next {
                        events.send(Err(e)).await.unwrap();
                    }
                },
            }
        })
        .detach();
        event_rx
    }
}
