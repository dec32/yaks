use reqwest::Client;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

use crate::{API_BASE, Result, range::Range};

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

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Post {
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    pub title: String,
}

impl Post {
    // TODO return username
    pub async fn collect(platform: &str, user_id: u64, range: Range) -> Result<Vec<Self>> {
        let client = Client::new();
        let mut posts = Vec::new();
        let mut offset = 0;
        loop {
            let url = format!("{API_BASE}/{platform}/user/{user_id}/posts-legacy?o={offset}");
            let payload = client
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .json::<Payload>()
                .await?;

            for post in payload.posts {
                if !range.contains(post.id) {
                    continue;
                }
                posts.push(post);
            }
            offset += payload.props.page_size;
            if offset > payload.props.count {
                break Ok(posts);
            }
        }
    }
}
