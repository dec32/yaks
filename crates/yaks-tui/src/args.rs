use anyhow::anyhow;
use clap::Parser;
use yaks_core::post::Range;

use crate::Result;

pub struct Args {
    pub platform: &'static str,
    pub user_id: u64,
    pub range: Range,
    pub cover: bool,
    pub out: &'static str,
    pub template: &'static str,
    pub jobs: usize,
}

impl Args {
    pub fn from_env() -> Result<Self> {
        RawArgs::parse().try_into()
    }
}

impl TryFrom<RawArgs> for Args {
    type Error = crate::Error;

    fn try_from(
        RawArgs {
            link,
            range,
            cover,
            out,
            template,
            jobs,
        }: RawArgs,
    ) -> std::result::Result<Self, Self::Error> {
        let range = range.unwrap_or("~".into()).parse()?;
        let split = link.split("/").collect::<Vec<_>>();
        let (platform, user_id) = if split.len() == 2 {
            (split[0].to_string().leak(), split[1].parse()?)
        } else {
            let Some(index) = split.iter().copied().position(|s| s == "user") else {
                return Err(anyhow!("Cannot parse link `{link}`"));
            };
            if index >= split.len() {
                return Err(anyhow!("Cannot parse link `{link}`"));
            }
            (
                split[index - 1].to_string().leak(),
                split[index + 1].parse()?,
            )
        };
        let out = out.leak();
        let template = template.leak();
        let args = Args {
            platform,
            user_id,
            range,
            cover,
            out,
            template,
            jobs,
        };
        Ok(args)
    }
}

#[derive(Parser, Debug)]
#[command(version, about = "Yet-another Kemono Scraper", long_about = None)]
struct RawArgs {
    /// URL of the page to download.
    /// Also accepts the format {platform}/{user_id} (e.g. fanbox/123456)
    #[arg(required = true)]
    link: String,

    /// Inclusive range of IDs of posts to download.
    /// Can be specified as {min}-{max}, {min}- or -{max}.
    #[arg(short, long)]
    range: Option<String>,

    /// Download cover images as well.
    #[arg(short, long)]
    cover: bool,

    /// Output directory for downloaded files.
    #[arg(short, long, default_value = "/mnt/c/Users/Administrator/Downloads")]
    out: String,

    /// Filename template for downloaded files.
    #[arg(long, default_value = "{user_id}/{post_id}_{index}")]
    template: String,

    /// Maximum amount of parallel downloading tasks.
    #[arg(short, long, default_value_t = 8)]
    jobs: usize,
}
