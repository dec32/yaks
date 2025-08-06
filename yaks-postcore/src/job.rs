use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use derive_more::Deref;
use serde::Deserialize;
use async_channel::{self, Receiver, Sender};
use tokio::fs;

use crate::{
    API_BASE, BRWOSE_INTERVAL, POST_BROWSERS, UserID, client,
    post::{Post, Profile},
};

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

pub fn create_jobs(
    posts: Vec<Post>,
    platform: &'static str,
    user_id: UserID,
    profile: Profile,
    cover: bool,
    out: &'static str,
    template: &'static str,
    errors: Sender<crate::Error>,
) -> Receiver<Job> {
    let (tx, rx) = async_channel::unbounded();
    // convert vec into chann (ok this is very silly)
    let (post_tx, post_rx) = async_channel::bounded(POST_BROWSERS);
    tokio::spawn(async move {
        for post in posts {
            post_tx.send(post).await.unwrap();
        }
    });
    let posts = post_rx;

    // browse post
    for _ in 0..POST_BROWSERS {
        let tx = tx.clone();
        let posts = posts.clone();
        let errors = errors.clone();
        tokio::spawn(async move {
            while let Ok(post) = posts.recv().await {
                let id = post.id;
                match browse(post, platform, user_id, profile, cover, out, template).await {
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
                tokio::time::sleep(BRWOSE_INTERVAL).await;
            }
        });
    }
    rx
}

async fn browse(
    Post { id, title }: Post,
    platform: &'static str,
    user_id: UserID,
    profile: Profile,
    cover: bool,
    dest: &'static str,
    template: &'static str,
) -> anyhow::Result<Vec<Job>> {
    #[derive(Debug, Deserialize)]
    struct Payload {
        previews: Vec<Preview>,
    }

    #[derive(Debug, Deserialize)]
    struct Preview {
        name: String,
        path: String,
        server: String,
    }
    let url = format!("{API_BASE}/{platform}/user/{user_id}/post/{id}");
    let payload = client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<Payload>()
        .await?;

    let mut jobs = Vec::new();
    for (index, Preview { name, path, server }) in payload
        .previews
        .iter()
        .enumerate()
        .skip(if cover { 0 } else { 1 })
    {
        let url = format!("{server}/data{path}").into_boxed_str();
        let name = PathBuf::from(name.replace("/", "／"));
        let mut location = template.to_string();
        if !location.ends_with("{filename}") {
            if let Some(ext) = name.extension() {
                location.push('.');
                location.push_str(ext.to_string_lossy().as_ref());
            }
        }
        // todo use runtime formatting library
        let location = location
            .replace("{user_id}", &user_id.to_string())
            .replace("{nickname}", &profile.nickname)
            .replace("{post_id}", &id.to_string())
            .replace("{index}", &index.to_string())
            .replace("{title}", &title)
            .replace("{filename}", name.to_string_lossy().as_ref());

        let mut out = PathBuf::from(dest).join(&location);
        // TODO: not the best way to check the exisitence of a file
        if fs::metadata(&out).await.is_err() {
            continue;
        }
        let filename = out
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            .into_boxed_str();
        // append .part to dest files for recovery
        out.pop();
        out.push(format!("{filename}.parts"));
        let out = out.into_boxed_path();
        let job = Job(Arc::new(JobRef { filename, url, out }));
        jobs.push(job);
    }
    Ok(jobs)
}
