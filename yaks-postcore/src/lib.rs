use std::{result, sync::OnceLock, time::Duration};

use reqwest::{Client, ClientBuilder};

pub mod engine;
mod job;
mod post;
mod worker;

pub type Result<T, E = Error> = result::Result<T, E>;
pub type Error = anyhow::Error;

// consts
pub const API_BASE: &str = "https://kemono.cr/api/v1";
pub const TIMEOUT: Duration = Duration::from_secs(30);
pub const TASK_CREATION_INTERVAL: Duration = Duration::from_millis(1000);
pub const POST_BROWSERS: usize = 5;

// static
pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| ClientBuilder::new().timeout(TIMEOUT).build().unwrap())
}

pub enum Event {}
