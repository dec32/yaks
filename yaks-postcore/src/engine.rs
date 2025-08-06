use std::ops::RangeInclusive;

use futures::FutureExt;
use async_channel::{self, Receiver};

use crate::{
    Event, UserID, job,
    post::{self, PostID},
    worker::{self, Prog},
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
        use Event as E;
        use Prog as P;

        // event chann (for TUI/GUI)
        let (events, event_rx) = async_channel::unbounded();

        // chann for Error
        let (error_tx, errors) = async_channel::unbounded();

        tokio::spawn(async move {
            let profile = match post::fetch_profile(platform, user_id).await {
                Ok(profile) => profile,
                Err(e) => {
                    error_tx.send(crate::Error::Profile(e)).await.unwrap();
                    return;
                }
            };
            let posts = match post::scrape_posts(platform, user_id, range).await {
                Ok(posts) => posts,
                Err(e) => {
                    error_tx.send(crate::Error::Scrape(e)).await.unwrap();
                    return;
                }
            };
            let jobs = job::create_jobs(
                posts,
                platform,
                user_id,
                profile,
                cover,
                out,
                template,
                error_tx.clone(),
            );
            log::info!("created job chann");
            let progress = worker::start_workers(workers, jobs.clone(), error_tx);
            log::info!("created progress chann");
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
        });
        event_rx
    }
}
