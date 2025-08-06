use std::{path::Path, sync::Arc};

use derive_more::Deref;
use smol::channel::{self, Sender};

use crate::{POST_BROWSERS, post::Post};

/// correspond to one single file in a post
#[derive(Debug, Clone, Deref)]
pub struct Job(Arc<JobRef>);

#[derive(Debug)]
pub struct JobRef {
    pub filename: Box<str>,
    pub url: Box<str>,
    pub out: Box<Path>,
}

pub type JobID = usize;

impl Job {
    #[inline(always)]
    pub fn id(&self) -> JobID {
        self.0.as_ref() as *const JobRef as *const () as usize
    }
}

pub fn create_jobs(posts: Vec<Post>, errors: Sender<crate::Error>) -> channel::Receiver<Job> {
    let (tx, rx) = channel::unbounded();
    // convert vec into chann (ok this is very silly)
    let (post_tx, post_rx) = channel::bounded(POST_BROWSERS);
    smol::spawn(async move {
        let mut posts = posts.into_iter();
        while let Some(post) = posts.next() {
            post_tx.send(post).await.unwrap();
        }
    })
    .detach();
    let posts = post_rx;

    // browse post
    for _ in 0..POST_BROWSERS {
        let tx = tx.clone();
        let posts = posts.clone();
        let errors = errors.clone();
        smol::spawn(async move {
            while let Ok(post) = posts.recv().await {
                let id = post.id;
                match browse(post).await {
                    Ok(jobs) => {
                        for job in jobs {
                            tx.send(job).await.unwrap();
                        }
                    }
                    Err(e) => {
                        let e = crate::Error::Browse(id, e);
                        errors.send(e).await.unwrap()
                    }
                }
            }
        })
        .detach();
    }
    rx
}

async fn browse(post: Post) -> anyhow::Result<Vec<Job>> {
    todo!()
}
