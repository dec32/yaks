use std::{result, sync::OnceLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub mod engine;
mod job;
mod post;
mod worker;

// re-exports
pub use engine::Engine;
pub use job::{Job, JobID};
pub use post::{Post, PostID, Profile};

// consts
pub(crate) const API_BASE: &str = "https://kemono.cr/api/v1";
pub(crate) const TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const BRWOSE_INTERVAL: Duration = Duration::from_millis(1000);
pub(crate) const POST_BROWSERS: usize = 5;

// static
pub(crate) fn client() -> &'static Client {
    static INSTANCE: OnceLock<Client> = OnceLock::new();
    INSTANCE.get_or_init(|| ClientBuilder::new().timeout(TIMEOUT).build().unwrap())
}

// types
pub type Result<T, E = crate::Error> = result::Result<T, E>;
pub type UserID = u64;

/// Event sent to the UI, by the engine, not the submodules.
///
/// Submodules should only sent data related to its job and
/// let the engine decide how to represent the data as events.
#[derive(Debug)]
pub enum Event {
    /// The profile of the artist has been fetched.
    Profile(Profile),
    /// A page of posts are scraped.
    Posts(usize),
    /// All pages are handled. No more post to offer.
    PostsExhausted,
    /// A job is created.
    Job(Job),
    /// All posts are browsed. No more jobs to create.
    JobExhausted,
    /// A job is added to the download queue.
    Enqueue(JobID),
    /// A job has setup its connection with the server.
    /// The file size (in bytes) is also offered.
    Init(JobID, u64),
    /// A job has received a chunk from the server.
    /// The chunk size (in bytes) is also offered.
    Chunk(JobID, u64),
    /// A job has been fully downloaded.
    Fin(JobID),
    /// All jobs are downloaded.
    Clear,
}

/// Possible errors, which may carry extra metadata.
///
/// The metadata is not included in the displayed String.
///
/// UI should decide how to represent the metadata within
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Profile(anyhow::Error),
    #[error(transparent)]
    Scrape(anyhow::Error),
    #[error("{1}")]
    Browse(PostID, anyhow::Error),
    #[error("{1}")]
    Download(Job, anyhow::Error),
}
