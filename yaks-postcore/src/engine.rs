use std::ops::RangeInclusive;

use futures::FutureExt;
use smol::channel::{self, Receiver};

use crate::{Event, job, post, worker};

pub struct Engine {}

impl Engine {
    pub fn start(
        platform: &'static str,
        user_id: u64,
        range: RangeInclusive<u64>,
        workers: u8,
    ) -> Receiver<crate::Result<Event>> {
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
            // listen for channels
            futures::select! {
                job = jobs.recv().fuse() => {
                    todo!()
                },
                progress = progress.recv().fuse() => {
                    todo!()
                },
                e = errors.recv().fuse() => {
                    events.send(Err(e.unwrap())).await.unwrap()
                },
            }
        })
        .detach();
        event_rx
    }
}
