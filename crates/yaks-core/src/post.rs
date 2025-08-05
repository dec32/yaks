use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use tokio::sync::mpsc::Sender;

use crate::{client, event::Event, range::Range, Result, API_BASE};

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
    pub async fn profile(platform: &str, user_id: u64) -> Result<&'static str> {
        #[derive(Deserialize)]
        struct Profile {
            name: String,
            #[allow(unused)]
            public_id: String,
        }
        let profile = client()
            .get(format!("{API_BASE}/{platform}/user/{user_id}/profile"))
            .send()
            .await?
            .error_for_status()?
            .json::<Profile>()
            .await?;
        Ok(profile.name.leak())
    }

    pub async fn scrape(
        platform: &str,
        user_id: u64,
        range: Range,
        tx: Sender<Event>,
    ) -> Result<Vec<Self>> {
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
            let mut inc = 0;
            for post in posts {
                if !range.contains(post.id) {
                    continue;
                }
                res.push(post);
                inc += 1;
            }
            tx.send(Event::MorePosts(inc)).await.unwrap();
            offset += page_size;
            if offset > count {
                tx.send(Event::NoMorePosts).await.unwrap();
                break Ok(res);
            }
        }
    }
}
