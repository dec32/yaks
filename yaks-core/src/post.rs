use std::ops::RangeInclusive;

use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use ustr::Ustr;

use crate::{API_BASE, SCRAPE_INTERVAL, UserID, client};

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Profile {
    #[serde(rename = "name")]
    pub nickname: Ustr,
    #[allow(unused)]
    #[serde(rename = "public_id")]
    pub username: Ustr,
}

/// Get the username of the artist
pub async fn fetch_profile(platform: &'static str, user_id: UserID) -> anyhow::Result<Profile> {
    let profile = client()
        .get(format!("{API_BASE}/{platform}/user/{user_id}/profile"))
        .send()
        .await?
        .error_for_status()?
        .json::<Profile>()
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
    platform: &'static str,
    user_id: UserID,
    range: RangeInclusive<PostID>,
) -> anyhow::Result<Vec<Post>> {
    #[derive(Debug, Deserialize)]
    struct Payload {
        #[serde(rename = "results")]
        posts: Vec<Post>,
        props: Props,
    }

    #[derive(Debug, Deserialize)]
    struct Props {
        #[serde(rename = "limit")]
        page_size: usize,
        count: usize,
    }
    let mut res = Vec::new();
    let mut offset = 0;
    loop {
        let url = format!("{API_BASE}/{platform}/user/{user_id}/posts-legacy?o={offset}");
        let Payload {
            posts,
            props: Props { page_size, count },
        } = client()
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        for post in posts {
            if !range.contains(&post.id) {
                continue;
            }
            res.push(post);
        }
        offset += page_size;
        if offset > count {
            break Ok(res);
        }
        tokio::time::sleep(SCRAPE_INTERVAL).await;
    }
}
