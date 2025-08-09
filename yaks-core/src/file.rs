use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_channel::{self, Receiver, Sender};
use derive_more::Deref;
use leaky::Leak;
use serde::Deserialize;
use tokio::fs;
use ustr::Ustr;
use yaks_common::SenderExt;

use crate::{
    API_BASE, BROWSE_INTERVAL, POST_BROWSERS, UserID, client,
    post::{Post, Profile},
};

/// correspond to one single file in a post
#[derive(Debug, Clone, Deref)]
pub struct File(Arc<FileRef>);

#[derive(Debug)]
pub struct FileRef {
    pub filename: Box<str>,
    pub url: Box<str>,
    pub dest: Box<Path>,
}

pub type FileID = usize;

impl File {
    #[inline(always)]
    pub fn id(&self) -> FileID {
        self.0.as_ref() as *const FileRef as *const () as usize
    }
}

pub fn collect_files(
    posts: Vec<Post>,
    platform: &'static str,
    user_id: UserID,
    profile: Profile,
    out: Leak<Path>,
    template: Leak<str>,
    errors: Sender<crate::Error>,
) -> Receiver<Vec<File>> {
    let (tx, rx) = async_channel::unbounded();
    // convert vec into chann (ok this is very silly)
    let (post_tx, post_rx) = async_channel::bounded(POST_BROWSERS);
    tokio::spawn(async move {
        for post in posts {
            post_tx.send_or_panic(post).await;
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
                match browse(post, platform, user_id, profile, out, template).await {
                    Ok(files) => {
                        tx.send_or_panic(files).await;
                    }
                    Err(e) => {
                        let e = crate::Error::Browse(id, e);
                        errors.send_or_panic(e).await;
                    }
                }
                tokio::time::sleep(BROWSE_INTERVAL).await;
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
    out: Leak<Path>,
    template: Leak<str>,
) -> anyhow::Result<Vec<File>> {
    #[derive(Debug, Deserialize)]
    #[serde(bound = "'de: 'body")]
    struct Payload<'body> {
        previews: Vec<Preview<'body>>,
    }

    #[derive(Debug, Deserialize)]
    struct Preview<'body> {
        #[serde(rename = "type")]
        typ: Ustr,
        /// some uploaded files don't have an original filename
        #[serde(default, rename = "name")]
        filename: &'body str,
        path: &'body str,
        server: Ustr,
    }
    if template.starts_with("/") {
        panic!("illegal template {template}");
    }
    let url = format!("{API_BASE}/{platform}/user/{user_id}/post/{id}");
    let bytes = client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let payload = serde_json::from_slice::<Payload>(&bytes)?;

    let mut files = Vec::new();
    for (
        index,
        Preview {
            filename: name,
            path,
            server,
            ..
        },
    ) in payload
        .previews
        .iter()
        .filter(|p| p.typ == "thumbnail")
        .enumerate()
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
            .replace("{username}", &profile.username)
            .replace("{nickname}", &profile.nickname)
            .replace("{post_id}", &id.to_string())
            .replace("{index}", &index.to_string())
            .replace("{title}", &title)
            .replace("{filename}", name.to_string_lossy().as_ref());

        let mut dest = out.join(&location);
        if fs::try_exists(&dest).await? {
            continue;
        }
        let filename = dest
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            .into_boxed_str();
        // append .part to dest files for recovery
        dest.pop();
        dest.push(format!("{filename}.parts"));
        let dest = dest.into_boxed_path();
        let file = File(Arc::new(FileRef {
            filename,
            url,
            dest,
        }));
        files.push(file);
    }
    Ok(files)
}
