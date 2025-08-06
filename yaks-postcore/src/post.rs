use std::ops::RangeInclusive;

use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

use crate::{API_BASE, client};

#[derive(Deserialize)]
pub struct Profile {
    #[serde(rename = "name")]
    pub nickname: String,
    #[allow(unused)]
    #[serde(rename = "public_id")]
    pub username: String,
}

/// Get the username of the artist
pub async fn fetch_profile(platform: &'static str, user_id: u64) -> anyhow::Result<Profile> {
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
    user_id: u64,
    profile: Profile,
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
    todo!()
}
