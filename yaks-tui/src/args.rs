use std::path::{Path, PathBuf};

use clap::Parser;
use leaky::Leak;
use yaks_common::Range;
use yaks_core::Conf;

pub struct Args {
    pub url: Leak<str>,
    pub range: Range,
    pub out: Leak<Path>,
    pub format: Leak<str>,
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
            ))?
            .into();
        let format = conf.format.unwrap_or(args.format).into();

        let workers = conf.jobs.unwrap_or(args.jobs);

        // only present in args
        let url = args.url.into();

        let range = if let Some(range) = args.range {
            range.parse()?
        } else {
            Range::default()
        };
        let args = Args {
            url,
            range,
            out,
            format,
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
    /// Can be specified as {min}~{max}, {min}~ or ~{max}
    #[arg(short, long)]
    range: Option<String>,
    /// Output directory for downloaded files [default: $HOME/Downloads]
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Filename format for downloaded files
    #[arg(short, long, default_value = "{nickname}/{post_id}_{index}")]
    format: String,
    /// Maximum amount of parallel jobs
    #[arg(short, long, default_value = "5")]
    jobs: u8,
}
