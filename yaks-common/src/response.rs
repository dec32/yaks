use std::io::Read;

use anyhow::bail;
use flate2::read::GzDecoder;
use reqwest::Response;
use serde::Deserialize;
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
        decode_using_strats(&bytes, [decode_plain, decode_gziped])
    }
}

fn decode_using_strats<I, F, T>(bytes: &[u8], strats: I) -> anyhow::Result<T>
where
    I: IntoIterator<Item = F>,
    F: FnOnce(&[u8]) -> anyhow::Result<T>,
    T: for<'de> Deserialize<'de>,
{
    let mut sink = Vec::new();
    for strat in strats {
        match strat(bytes) {
            Ok(decoded) => {
                return Ok(decoded);
            }
            Err(e) => {
                sink.push(e);
                continue;
            }
        }
    }
    bail!("{sink:?}")
}

fn decode_plain<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    Ok(serde_json::from_slice(bytes)?)
}

fn decode_gziped<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut decoder = GzDecoder::new(bytes);
    let mut body = String::new();
    decoder.read_to_string(&mut body)?;
    Ok(serde_json::from_str(&body)?)
}

#[allow(unused)]
fn decode_malformed<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let malformed = String::from_utf8(bytes.to_vec())?;
    Ok(serde_json::from_str(&malformed)?)
}
