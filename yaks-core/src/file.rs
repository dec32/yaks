use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_channel::{self, Receiver, Sender};
use derive_more::Deref;
use leaky::Leak;
use serde::Deserialize;
use tokio::{fs, io::AsyncWriteExt};
use ustr::Ustr;
use yaks_common::{ResponseExt, SenderExt, StrExt};

use crate::{
    API_BASE, BROWSE_INTERVAL, POST_BROWSERS, client,
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
    profile: Profile,
    out: PathBuf,
    format: String,
    save_text: bool,
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
    // TODO introduce structured concurrency here so tha
    // we don't need to leak `format`, `out` and the fields of `Profile`
    let format = format
        .replace("\\", "/")
        .trim_start_matches('/')
        .to_string()
        .into();
    let out = out.into();
    for _ in 0..POST_BROWSERS {
        let tx = tx.clone();
        let posts = posts.clone();
        let errors = errors.clone();

        tokio::spawn(async move {
            while let Ok(post) = posts.recv().await {
                let id = post.id;
                match browse(post, profile, out, format, save_text).await {
                    Ok(files) => {
                        tx.send_or_panic(files).await;
                    }
                    Err(e) => {
                        let e = crate::Error::Browse(id, e);
                        errors.send_or_panic(e).await;
                    }
                }
                tokio::time::sleep(BROWSE_INTERVAL.get()).await;
            }
        });
    }
    rx
}

async fn browse(
    Post { id, title }: Post,
    Profile {
        platform,
        user_id,
        nickname,
        username,
        post_count: _,
    }: Profile,
    out: Leak<Path>,
    format: Leak<str>,
    save_text: bool,
) -> anyhow::Result<Vec<File>> {
    #[derive(Debug, Deserialize)]
    struct Payload {
        previews: Vec<Preview>,
        post: BrowsablePost,
    }

    #[derive(Debug, Deserialize)]
    struct Preview {
        #[serde(rename = "type")]
        ty: Ustr,
        #[serde(default, rename = "name")]
        filename: String,
        #[serde(default)]
        path: String,
        #[serde(default)]
        server: Ustr,
    }

    #[derive(Debug, Deserialize)]
    struct BrowsablePost {
        #[serde(default, rename = "content")]
        text: String,
    }

    let title = title.to_path_safe();
    let nickname = nickname.to_path_safe();
    let username = username.to_path_safe();

    let url = format!("{API_BASE}/{platform}/user/{user_id}/post/{id}");
    let payload = client()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .sneaky_json::<Payload>()
        .await?;

    // ---------------------------------------------------------
    // save the text of the post
    // ---------------------------------------------------------
    if save_text && !payload.post.text.is_empty() {
        // save as markdown
        let text = htmd::convert(&payload.post.text).unwrap_or(payload.post.text);
        // find the last {post_id}/{title}
        let post_id_end = format.find("{post_id}").map(|i| i + "{post_id}".len());
        let title_end = format.find("{title}").map(|i| i + "{title}".len());
        let end = post_id_end.into_iter().chain(title_end).max();

        let format = if let Some(end) = end {
            let append = match format[end..].chars().next() {
                // user arrange posts into separate folders
                Some('/') => "/post.md",
                // user arrange posts under one big folder
                _ => ".md",
            };
            let mut format = format[..end].to_string();
            format.push_str(append);
            format
        } else {
            // post-level meta is missing.
            let mut format = format.to_string();
            format.push_str("{post_id}_{title}.md");
            format
        };

        let dest = format
            .replace("{user_id}", &user_id)
            .replace("{post_id}", &id.to_string())
            .replace("{username}", &username)
            .replace("{nickname}", &nickname)
            .replace("{title}", &title);
        let dest = out.join(dest);
        if !fs::try_exists(&dest).await? {
            let mut dest = {
                let parent = dest.parent().unwrap();
                fs::create_dir_all(parent).await?;
                fs::File::create(dest).await?
            };
            dest.write_all(text.as_bytes()).await?;
        }
    }

    // ---------------------------------------------------------
    // collect the files
    // ---------------------------------------------------------
    let mut files = Vec::new();
    for (
        index,
        Preview {
            filename,
            path,
            server,
            ..
        },
    ) in payload
        .previews
        .into_iter()
        .filter(|p| p.ty == "thumbnail")
        .enumerate()
    {
        let filename = filename.to_path_safe();
        let url = format!("{server}/data{path}").into_boxed_str();
        let mut format = format.to_string();
        if !format.ends_with("{filename}")
            && let Some(ext) = Path::new(filename.as_ref()).extension()
        {
            format.push('.');
            format.push_str(ext.to_string_lossy().as_ref());
        }
        // todo use runtime formatting library
        let dest = format
            .replace("{user_id}", &user_id)
            .replace("{post_id}", &id.to_string())
            .replace("{index}", &index.to_string())
            .replace("{username}", &username)
            .replace("{nickname}", &nickname)
            .replace("{title}", &title)
            .replace("{filename}", &filename);

        let mut dest = out.join(&dest);
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
