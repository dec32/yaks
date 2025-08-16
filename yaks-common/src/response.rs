use flate2::read::GzDecoder;
use reqwest::Response;
use serde::Deserialize;
use std::io::Read;
pub trait ResponseExt {
    fn sneaky_json<T>(self) -> impl Future<Output = anyhow::Result<T>>
    where
        for<'de> T: Deserialize<'de>;
}

impl ResponseExt for Response {
    async fn sneaky_json<T>(self) -> anyhow::Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let bytes = self.bytes().await?;
        match serde_json::from_slice(&bytes) {
            Ok(json) => return Ok(json),
            Err(_e) => {
                let mut decoder = GzDecoder::new(&*bytes);
                let mut body = String::new();
                decoder.read_to_string(&mut body)?;
                let json = serde_json::from_str(&body)?;
                Ok(json)
            }
        }
    }
}
