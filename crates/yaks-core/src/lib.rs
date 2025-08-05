use std::time::Duration;

pub type Result<T = (), E = crate::Error> = std::result::Result<T, E>;
pub type Error = anyhow::Error;

pub mod engine;
pub mod event;
pub mod post;
pub mod range;
pub mod task;

pub const API_BASE: &str = "https://kemono.cr/api/v1";
pub const PAGE_SIZE: u64 = 50;
pub const COMMON_TIMEOUT: Duration = Duration::from_secs(30);
