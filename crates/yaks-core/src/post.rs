use std::{str::FromStr, time::Duration, u64};

use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use tokio::time::sleep;

use crate::{API_BASE, PAGE_SIZE, Result};

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Post {
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub user: u64,
    pub title: String,
}

pub struct Range(u64, u64);

impl Range {
    pub fn contains(&self, value: u64) -> bool {
        self.0 <= value && value <= self.1
    }
}

impl Default for Range {
    fn default() -> Self {
        Self(0, u64::MAX)
    }
}

impl FromStr for Range {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let split = s
            .split_once("-")
            .ok_or(anyhow!("Illegal range literal {s}."))?;
        let start = if split.0.is_empty() {
            0
        } else {
            split.0.parse()?
        };
        let end = if split.1.is_empty() {
            u64::MAX
        } else {
            split.1.parse()?
        };
        Ok(Self(start, end))
    }
}

#[derive(Debug, Deserialize)]
struct Payload {
    results: Vec<Post>,
}

impl Post {
    // TODO return username
    pub async fn collect(platform: &str, user_id: u64, range: Range) -> Result<Vec<Self>> {
        let mut posts = Vec::new();
        let client = Client::new();
        let mut offset = 0;
        loop {
            let url = format!("{API_BASE}/{platform}/user/{user_id}/posts-legacy?o={offset}");
            log::info!("Scrapping page: {}", offset / PAGE_SIZE + 1);
            let resp = client.get(&url).send().await?;
            let status = resp.status();
            if !status.is_success() {
                // todo: offset 过大时也会返回错误
                break;
            }

            let payload = resp.json::<Payload>().await?;
            if payload.results.is_empty() {
                break;
            }

            for post in payload.results {
                if !range.contains(post.id) {
                    log::info!("Skipped out-of-range post {}", post.id);
                    continue;
                }
                posts.push(post);
            }
            sleep(Duration::from_millis(300)).await;
            offset += PAGE_SIZE;
        }

        Ok(posts)
    }
}
