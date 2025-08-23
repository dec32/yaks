use std::path::Path;

use async_channel::{self, Receiver, Sender};
use leaky::Leak;
use yaks_common::{Range, SenderExt};

use crate::{
    Event, File, FileID, file,
    post::{self},
    worker::{self, Prog},
};

#[derive(Default)]
pub struct Engine {}

impl Engine {
    pub fn start(
        self,
        url: Leak<str>,
        range: Range,
        out: Leak<Path>,
        fmt: Leak<str>,
        save_text: bool,
        workers: u8,
    ) -> Receiver<crate::Result<Event>> {
        // event chann (for TUI/GUI)
        let (events, event_rx) = async_channel::unbounded();
        // chann for Error
        let (error_tx, errors) = async_channel::unbounded();
        listen_errors(errors, events.clone());

        tokio::spawn(async move {
            // parsing url
            let (platform, user_id) = match post::parse_url(url) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error_tx.send_or_panic(crate::Error::Profile(e)).await;
                    return;
                }
            };
            // fetching profile
            let profile = match post::fetch_profile(platform, user_id).await {
                Ok(profile) => profile,
                Err(e) => {
                    error_tx.send_or_panic(crate::Error::Profile(e)).await;
                    return;
                }
            };
            events.send_or_panic(Ok(Event::Profile(profile))).await;
            // scrape all posts
            let posts = match post::scrape_posts(platform, user_id, profile.post_count, range).await
            {
                Ok(posts) => {
                    events.send_or_panic(Ok(Event::Posts(posts.len()))).await;
                    posts
                }
                Err(e) => {
                    println!("Error creating posts {e}");
                    error_tx.send_or_panic(crate::Error::Scrape(e)).await;
                    return;
                }
            };
            events.send_or_panic(Ok(Event::PostsExhausted)).await;
            // collect files. each file will have two copies. one for download and one for UI.
            let files_rx =
                file::collect_files(posts, profile, out, fmt, save_text, error_tx.clone());
            let files = listen_files(files_rx, events.clone());
            // download
            let progress = worker::start_workers(workers, files.clone(), error_tx);
            listen_prog(progress, events);
        });
        event_rx
    }
}

fn listen_errors(errors: Receiver<crate::Error>, events: Sender<crate::Result<Event>>) {
    tokio::spawn(async move {
        while let Ok(e) = errors.recv().await {
            events.send_or_panic(Err(e)).await;
        }
    });
}

fn listen_files(
    files_rx: Receiver<Vec<File>>,
    events: Sender<crate::Result<Event>>,
) -> Receiver<File> {
    let (tx, rx) = async_channel::unbounded();
    tokio::spawn(async move {
        while let Ok(files) = files_rx.recv().await {
            for file in files.iter().cloned() {
                tx.send_or_panic(file).await;
            }
            events.send_or_panic(Ok(Event::Files(files))).await;
        }
        events.send_or_panic(Ok(Event::FilesExhausted)).await;
    });
    rx
}

fn listen_prog(prog: Receiver<(FileID, Prog)>, events: Sender<crate::Result<Event>>) {
    tokio::spawn(async move {
        while let Ok((id, prog)) = prog.recv().await {
            let event = match prog {
                Prog::Enqueue => Event::Enqueue(id),
                Prog::Init(size) => Event::Init(id, size),
                Prog::Chunk(size) => Event::Chunk(id, size),
                Prog::Fin => Event::Fin(id),
            };
            events.send_or_panic(Ok(event)).await;
        }
        events.send_or_panic(Ok(Event::Clear)).await;
    });
}
