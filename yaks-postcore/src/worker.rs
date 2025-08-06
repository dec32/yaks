use async_stream::try_stream;
use smol::{
    channel::{self, Receiver},
    pin,
    stream::{self, Stream, StreamExt},
};

use crate::job::{Job, JobID};

pub enum Progress {
    Established(usize),
    Chunk(usize),
    Fin,
}

/// Start a fixed number of workers.
/// The workers will drain the jobs from the receiver
/// and report the progress in to the `progress` sender
pub fn start_workers(workers: u8, jobs: channel::Receiver<Job>) -> Receiver<(JobID, Progress)> {
    let (tx, rx) = channel::unbounded();
    for _ in 0..workers {
        let jobs = jobs.clone();
        let progress = tx.clone();
        smol::spawn(async move {
            work(jobs, progress).await;
        })
        .detach();
    }
    rx
}

/// download the given jobs subsquentially using streams.
/// the function consumes the streams and:
/// 1. report progress in to the sender
/// 2. capture yielded errors and send them... somewhere?
async fn work(jobs: channel::Receiver<Job>, tx: channel::Sender<(JobID, Progress)>) {
    // TODO: why is it not Option?
    while let Ok(job) = jobs.recv().await {
        let id = job.id();
        let stream = download(job);
        pin!(stream);
        while let Some(progress) = stream.next().await {
            match progress {
                Ok(progress) => tx.send((id, progress)).await.unwrap(),
                Err(e) => todo!("where to send this error?"),
            }
        }
    }
}

/// return a stream of progress (and errors some time)
fn download(job: Job) -> impl Stream<Item = crate::Result<Progress>> {
    try_stream! {

        yield Progress::Fin;
    }
}
