use std::{sync::OnceLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub type Result<T = (), E = crate::Error> = std::result::Result<T, E>;
pub type Error = anyhow::Error;

pub mod engine;
pub mod event;
pub mod post;
pub mod range;
pub mod task;

// consts
pub const API_BASE: &str = "https://kemono.cr/api/v1";
pub const TIMEOUT: Duration = Duration::from_secs(30);
pub const TASK_CREATION_INTERVAL: Duration = Duration::from_millis(1000);
pub const TASK_CREATION_BATCH_SIZE: usize = 5;

// static
pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| ClientBuilder::new().timeout(TIMEOUT).build().unwrap())
}
