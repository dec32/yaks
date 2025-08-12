use anyhow::anyhow;
use async_channel::{Receiver, Sender};
use async_stream::try_stream;
use futures::{Stream, StreamExt};
use tokio::{
    fs::{self},
    io::AsyncWriteExt,
    pin,
};
use yaks_common::SenderExt;

use crate::{FileID, client, file::File};

#[derive(Debug)]
pub enum Prog {
    Enqueue,
    Init(u64),
    Chunk(u64),
    Fin,
}

/// Start a fixed number of workers.
/// The workers will drain the files from the receiver
/// and report the progress in to the `progress` sender
pub fn start_workers(
    workers: u8,
    files: Receiver<File>,
    errors: Sender<crate::Error>,
) -> Receiver<(FileID, Prog)> {
    let (tx, rx) = async_channel::unbounded();
    for _ in 0..workers {
        let files = files.clone();
        let progress = tx.clone();
        let errors = errors.clone();
        tokio::spawn(async move {
            work(files, progress, errors).await;
        });
    }
    rx
}

/// download the given files subsquentially using streams.
/// the function drains the streams and:
/// 1. report progress in to the sender
/// 2. capture yielded errors and send them... somewhere?
async fn work(files: Receiver<File>, tx: Sender<(FileID, Prog)>, errors: Sender<crate::Error>) {
    while let Ok(file) = files.recv().await {
        let id = file.id();
        let stream = download(file.clone());
        pin!(stream);
        tx.send_or_panic((id, Prog::Enqueue)).await;
        while let Some(progress) = stream.next().await {
            match progress {
                // todo: too much clone here
                Ok(progress) => tx.send_or_panic((id, progress)).await,
                Err(e) => {
                    let e = crate::Error::Download(id, e);
                    errors.send_or_panic(e).await;
                    break;
                }
            }
        }
    }
}

/// return a stream of progress (and errors some time)
fn download(file: File) -> impl Stream<Item = anyhow::Result<Prog>> {
    try_stream! {
        // setting up the output file and the http response
        let parent = file.dest.parent().unwrap();
        let mut dest = {
            fs::create_dir_all(parent).await?;
            fs::File::create(&file.dest).await?
        };
        let mut resp = client()
            .get(file.url.as_ref())
            .send()
            .await?
            .error_for_status()?;
        let total = resp
            .content_length()
            .ok_or(anyhow!("content-length is missing"))?;
        yield Prog::Init(total);
        // download by chunks
        loop {
            match resp.chunk().await? {
                Some(chunk) => {
                    dest.write_all(&chunk).await?;
                    yield Prog::Chunk(chunk.len() as u64);
                }
                None => {
                    let real_dest = parent.join(file.filename.as_ref());
                    fs::rename(&file.dest, real_dest).await?;
                    break;
                }
            };
        }
        yield Prog::Fin;
    }
}
