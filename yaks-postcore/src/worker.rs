use async_stream::try_stream;
use futures::{Stream, StreamExt};
use async_channel::{Receiver, Sender};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    pin,
};

use crate::{
    client,
    job::{Job, JobID},
};

#[derive(Debug)]
pub enum Prog {
    Init(u64),
    Chunk(u64),
    Fin,
}

/// Start a fixed number of workers.
/// The workers will drain the jobs from the receiver
/// and report the progress in to the `progress` sender
pub fn start_workers(
    workers: u8,
    jobs: Receiver<Job>,
    errors: Sender<crate::Error>,
) -> Receiver<(JobID, Prog)> {
    let (tx, rx) = async_channel::unbounded();
    for _ in 0..workers {
        let jobs = jobs.clone();
        let progress = tx.clone();
        let errors = errors.clone();
        tokio::spawn(async move {
            work(jobs, progress, errors).await;
        });
    }
    rx
}

/// download the given jobs subsquentially using streams.
/// the function drains the streams and:
/// 1. report progress in to the sender
/// 2. capture yielded errors and send them... somewhere?
async fn work(
    jobs: Receiver<Job>,
    tx: Sender<(JobID, Prog)>,
    errors: Sender<crate::Error>,
) {
    while let Ok(job) = jobs.recv().await {
        let id = job.id();
        let stream = download(job.clone());
        pin!(stream);
        while let Some(progress) = stream.next().await {
            match progress {
                Ok(progress) => tx.send((id, progress)).await.unwrap(),
                Err(e) => {
                    let e = crate::Error::Download(job, e);
                    errors.send(e).await.unwrap();
                    break;
                }
            }
        }
    }
}

/// return a stream of progress (and errors some time)
fn download(job: Job) -> impl Stream<Item = anyhow::Result<Prog>> {
    try_stream! {
        // setting up the output file and the http response
        let parent = job.out.parent().unwrap();
        let mut dest = {
            fs::create_dir_all(parent).await?;
            File::create(&job.out).await?
        };
        let mut resp = client()
            .get(job.url.as_ref())
            .send()
            .await?
            .error_for_status()?;
        let total = resp
            .content_length()
            .ok_or(anyhow::anyhow!("content-length is missing"))?;
        yield Prog::Init(total);
        // download by chunks
        let mut cur = 0;
        loop {
            match resp.chunk().await? {
                Some(chunk) => {
                    dest.write_all(&chunk).await?;
                    cur += chunk.len() as u64;
                    yield Prog::Chunk(cur);
                }
                None => {
                    let real_path = parent.join(job.filename.as_ref());
                    fs::rename(&job.out, real_path).await?;
                    break;
                }
            };
        }
        yield Prog::Fin;
    }
}
