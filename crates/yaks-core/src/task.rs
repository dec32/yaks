use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use derive_more::derive::Deref;
use reqwest::Client;
use serde::Deserialize;
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    sync::mpsc::Sender,
    task::JoinSet,
};

use crate::{API_BASE, Result, TIMEOUT_FOR_PREP, TIMEOUT_FOR_START, event::Event, post::Post};

/// A read-only view of tasks that is cheap to clone
/// along threads.
#[derive(Clone, Deref)]
pub struct Task(Arc<TaskData>);

/// The numeric value of the inner pointer of `Task` as an ID.
pub type TaskID = usize;
impl Task {
    #[inline(always)]
    pub fn id(&self) -> TaskID {
        self.0.as_ref() as *const TaskData as *const () as TaskID
    }
}

pub struct TaskData {
    pub filename: Box<str>,
    pub url: Box<str>,
    pub out: Box<Path>,
}

impl Task {
    // todo throw events
    pub async fn prep(
        posts: Vec<Post>,
        cover: bool,
        out: &'static str,
        template: &'static str,
    ) -> Result<VecDeque<Task>> {
        let mut posts = posts;
        let batch_size = 6;
        let mut tasks = VecDeque::new();
        let mut set = JoinSet::new();
        while !posts.is_empty() {
            for _ in 0..batch_size {
                let Some(post) = posts.pop() else {
                    break;
                };
                set.spawn(Self::prep_one(post, cover, out, template));
            }
        }
        while let Some(res) = set.join_next().await {
            let new_tasks = res??;
            tasks.extend(new_tasks.into_iter());
        }
        Ok(tasks)
    }

    pub async fn prep_one(
        Post { id, user, title }: Post,
        cover: bool,
        dest: &'static str,
        template: &'static str,
    ) -> Result<Vec<Task>> {
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

        let client = Client::new();
        let url = format!("{API_BASE}/fanbox/user/{user}/post/{id}");
        let payload = client
            .get(&url)
            .timeout(TIMEOUT_FOR_PREP)
            .send()
            .await?
            .error_for_status()?
            .json::<Payload>()
            .await?;

        let mut tasks = Vec::new();
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
                .replace("{user_id}", &user.to_string())
                .replace("{post_id}", &id.to_string())
                .replace("{index}", &index.to_string())
                .replace("{title}", &title)
                .replace("{filename}", name.to_string_lossy().as_ref());

            let mut out = PathBuf::from(dest).join(&location);
            if fs::try_exists(&out).await? {
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
            let tmp = out.into_boxed_path();
            let task = Task(Arc::new(TaskData {
                filename,
                url,
                out: tmp,
            }));
            tasks.push(task);
        }
        Ok(tasks)
    }

    pub async fn start(self, tx: Sender<Event>) {
        if let Err(err) = self.clone()._start(tx.clone()).await {
            tx.send(Event::Fail(self.id(), err)).await.unwrap();
        }
    }

    async fn _start(self, tx: Sender<Event>) -> anyhow::Result<()> {
        tx.send(Event::Enqueue(self.clone())).await.unwrap();
        // setting up the output file and the http response
        let mut dest = {
            if let Some(parent) = self.out.parent() {
                fs::create_dir_all(parent).await?;
            }
            File::create(&self.out).await?
        };
        let client = Client::new();
        let mut resp = client
            .get(self.url.as_ref())
            .timeout(TIMEOUT_FOR_START)
            .send()
            .await?
            .error_for_status()?;
        let total = resp
            .content_length()
            .ok_or(anyhow!("content-length is missing"))?;
        tx.send(Event::Start(self.id(), total)).await.unwrap();
        // download by chunks
        let mut cur = 0;
        loop {
            let event = match resp.chunk().await? {
                Some(chunk) => {
                    dest.write_all(&chunk).await?;
                    cur += chunk.len() as u64;
                    Event::Updated(self.id(), cur)
                }
                None => {
                    let real_path = self.out.parent().unwrap().join(self.filename.as_ref());
                    fs::rename(&self.out, real_path).await?;
                    Event::Finished(self.id())
                }
            };
            let stopped = matches!(event, Event::Fail(..) | Event::Finished(..));
            tx.send(event).await.unwrap();
            if stopped {
                break Ok(());
            }
        }
    }
}
