use std::{
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use clap::Parser;
use yaks_core::{Conf, PostID};

pub struct Args {
    pub url: &'static str,
    pub range: RangeInclusive<PostID>,
    pub out: &'static Path,
    pub template: &'static str,
    pub workers: u8,
}

impl Args {
    pub async fn from_conf_then_env() -> anyhow::Result<Self> {
        let conf = Conf::load().await?;
        let args = RawArgs::parse();
        // configurable ones
        // todo: let clap handle dir_next::download_dir()
        let out = conf
            .out
            .or(args.out)
            .or_else(|| dirs_next::download_dir())
            .ok_or(anyhow::anyhow!(
                "Can not locate the default download folder"
            ))?;
        // where is my PathBuf::leak dear Rust team?
        let out = out
            .to_str()
            .ok_or(anyhow::anyhow!("Unrecognizable out path."))?
            .to_string()
            .leak();
        let out = Path::new(out);
        let template = conf.template.unwrap_or(args.template).leak();

        let workers = conf.jobs.unwrap_or(args.jobs);

        // only present in args
        let url = args.url.leak();
        let (start, end) = args
            .range
            .map(|s| s.leak().split_once("~"))
            .map(|o| o.ok_or(anyhow::anyhow!("Ranges are split by ~")))
            .unwrap_or(Ok(("", "")))?;

        let start = if start.is_empty() { 0 } else { start.parse()? };
        let end = if end.is_empty() {
            PostID::MAX
        } else {
            end.parse()?
        };
        let range = RangeInclusive::new(start, end);
        let args = Args {
            url,
            range,
            out,
            template,
            workers,
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
    url: String,
    /// Inclusive range of IDs of posts to download.
    /// Can be specified as {min}-{max}, {min}- or -{max}.
    #[arg(short, long)]
    range: Option<String>,
    /// Output directory for downloaded files.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Filename template for downloaded files.
    #[arg(long, default_value = "{nickname}/{post_id}_{index}")]
    template: String,
    /// Maximum amount of parallel jobs.
    #[arg(short, long, default_value = "5")]
    jobs: u8,
}
