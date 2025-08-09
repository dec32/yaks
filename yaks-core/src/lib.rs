use std::{result, sync::OnceLock, time::Duration};

use reqwest::{Client, ClientBuilder};

mod conf;
mod engine;
mod file;
mod post;
mod worker;

// re-exports
pub use conf::Conf;
pub use engine::Engine;
pub use file::{File, FileID};
pub use post::{Post, PostID, Profile};
use ustr::Ustr;

// consts
pub(crate) const API_BASE: &str = "https://kemono.cr/api/v1";
pub(crate) const TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const SCRAPE_INTERVAL: Duration = Duration::from_millis(500);
pub(crate) const BROWSE_INTERVAL: Duration = Duration::from_millis(500);
pub(crate) const BROWSE_RETRY_AFTER: Duration = Duration::from_secs(120);
pub(crate) const BROWSE_RETRY_TIMES: u8 = 3;
pub(crate) const POST_BROWSERS: usize = 5;

// static
pub(crate) fn client() -> &'static Client {
    static INSTANCE: OnceLock<Client> = OnceLock::new();
    INSTANCE.get_or_init(|| ClientBuilder::new().timeout(TIMEOUT).build().unwrap())
}

// types
pub type Result<T, E = crate::Error> = result::Result<T, E>;
pub type UserID = Ustr;

/// Event sent to the UI, by the engine, not the submodules.
///
/// Submodules should only sent data related to its file and
/// let the engine decide how to represent the data as events.
#[derive(Debug)]
pub enum Event {
    /// The profile of the artist has been fetched.
    Profile(Profile),
    /// A page of posts are scraped.
    Posts(usize),
    /// All pages are handled. No more post to offer.
    PostsExhausted,
    /// Files from a post are collected.
    Files(Vec<File>),
    /// All posts are browsed. No more file to collect.
    FilesExhausted,
    /// A file is added to the download queue.
    Enqueue(FileID),
    /// A file has setup its connection with the server.
    /// The total size (in bytes) is also offered.
    Init(FileID, u64),
    /// A file has received a chunk from the server.
    /// The chunk size (in bytes) is also offered.
    Chunk(FileID, u64),
    /// A file has been fully downloaded.
    Fin(FileID),
    /// All files are downloaded.
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
    Download(FileID, anyhow::Error),
}
