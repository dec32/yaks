use anyhow::bail;
use leaky::Leak;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use yaks_common::{Range, ResponseExt};

use crate::{
    API_BASE, BROWSE_RETRY_AFTER, BROWSE_RETRY_TIMES, PAGE_SIZE, SCRAPE_INTERVAL, UserID, client,
};

pub fn parse_url(url: Leak<str>) -> anyhow::Result<(Leak<str>, UserID)> {
    let split = url
        .split("?")
        .next()
        .unwrap()
        .split("/")
        .collect::<Vec<_>>();
    let (platform, user_id) = if split.len() == 2 {
        (split[0], split[1])
    } else {
        let Some(index) = split.iter().copied().position(|s| s == "user") else {
            bail!("Cannot parse URL `{}`", url);
        };
        if index >= split.len() {
            bail!("Cannot parse URL `{}`", url);
        }
        (split[index - 1], split[index + 1])
    };
    Ok((platform, user_id))
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Profile {
    #[serde(rename = "name")]
    pub nickname: Leak<str>,
    #[allow(unused)]
    #[serde(rename = "public_id")]
    pub username: Leak<str>,
    pub post_count: usize,
}

/// Get the username of the artist
pub async fn fetch_profile(platform: Leak<str>, user_id: UserID) -> anyhow::Result<Profile> {
    let profile = client()
        .get(format!("{API_BASE}/{platform}/user/{user_id}/profile"))
        .send()
        .await?
        .error_for_status()?
        .sneaky_json::<Profile>()
        .await?;
    Ok(profile)
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Post {
    #[serde_as(as = "DisplayFromStr")]
    pub id: PostID,
    pub title: String,
}

pub type PostID = u64;

pub async fn scrape_posts(
    platform: Leak<str>,
    user_id: UserID,
    post_count: usize,
    range: Range,
) -> anyhow::Result<Vec<Post>> {
    let mut res = Vec::new();
    let mut offset = 0;
    'quit: loop {
        let url = format!("{API_BASE}/{platform}/user/{user_id}/posts?o={offset}");
        let posts: Vec<Post> = {
            let mut retry = 0;
            loop {
                let resp = client().get(&url).send().await?;
                if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                    if retry >= BROWSE_RETRY_TIMES {
                        break resp.error_for_status()?.sneaky_json().await?;
                    } else {
                        retry += 1;
                        tokio::time::sleep(BROWSE_RETRY_AFTER).await;
                        continue;
                    }
                } else {
                    break resp.error_for_status()?.sneaky_json().await?;
                };
            }
        };

        for post in posts {
            if post.id > range {
                continue;
            }
            if post.id < range {
                break 'quit;
            }
            res.push(post);
        }
        offset += PAGE_SIZE;
        if offset > post_count {
            break;
        }
        tokio::time::sleep(SCRAPE_INTERVAL).await;
    }
    Ok(res)
}
