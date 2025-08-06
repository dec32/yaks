use std::{result, sync::OnceLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub mod engine;
mod job;
mod post;
mod worker;

pub use engine::Engine;

use crate::{job::Job, post::PostID};
pub type Result<T, E = crate::Error> = result::Result<T, E>;

// consts
pub const API_BASE: &str = "https://kemono.cr/api/v1";
pub const TIMEOUT: Duration = Duration::from_secs(30);
pub const TASK_CREATION_INTERVAL: Duration = Duration::from_millis(1000);
pub const POST_BROWSERS: usize = 5;

// static
pub fn client() -> &'static Client {
    static INSTANCE: OnceLock<Client> = OnceLock::new();
    INSTANCE.get_or_init(|| ClientBuilder::new().timeout(TIMEOUT).build().unwrap())
}

/// Event sent to the UI, by the engine, not the submodules.
///
/// Submodules should only sent data related to its job and
/// let the engine decide how to represent the data as events.
pub enum Event {}

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
