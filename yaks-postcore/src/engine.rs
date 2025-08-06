use std::ops::RangeInclusive;

use futures::FutureExt;
use smol::channel::{self, Receiver};

use crate::{Event, job, post, worker};

pub struct Engine {}

impl Engine {
    pub async fn start(
        platform: &'static str,
        user_id: u64,
        range: RangeInclusive<u64>,
        workers: u8,
    ) -> Receiver<crate::Result<Event>> {
        // event chann (for TUI/GUI)
        let (events, mut event_rx) = channel::unbounded();

        // chann for Error
        let (error_tx, mut errors) = channel::unbounded::<crate::Error>();

        // main logic
        let profile = post::fetch_profile(platform, user_id)
            .await
            .expect("where to send the errors?");
        let posts = post::scrape_posts(platform, user_id, profile, range)
            .await
            .expect("where to send the errors?");
        let jobs = job::create_jobs(posts);
        let progress = worker::start_workers(workers, jobs.clone());

        // listen for channels
        futures::select! {
            job = jobs.recv().fuse() => {
                todo!()
            },
            progress = progress.recv().fuse() => {
                todo!()
            },
            e = errors.recv().fuse() => {
                todo!()
            },
        }
        event_rx
    }
}
