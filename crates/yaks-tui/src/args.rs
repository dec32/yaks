use anyhow::anyhow;
use argh::FromArgs;
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
        argh::from_env::<RawArgs>().try_into()
    }
}

#[derive(Debug, FromArgs)]
#[argh(description = "args for Yakd")]
struct RawArgs {
    #[argh(
        positional,
        description = "URL of the page to download. Also accepts the format `{{platform}}/{{user_id}}` (e.g. `fanbox/123456`)"
    )]
    link: String,
    #[argh(
        option,
        default = "default_range()",
        description = "inclusive range of IDs of posts to download. Can be specified as `{{start}}-{{end}}`, `{{start}}-` or `-{{end}}`."
    )]
    range: String,
    #[argh(switch, description = "download cover images as well.")]
    cover: bool,
    #[argh(
        option,
        default = "default_out()",
        description = "output directory for downloaded files."
    )]
    out: String,
    #[argh(
        option,
        default = "default_template()",
        description = "filename template of downloaded files. Defaults to `{{user}}/{{post_id}}_{{index}}`."
    )]
    template: String,
    #[argh(
        option,
        default = "default_jobs()",
        description = "maximum amount of parallel downloading tasks. Defaults to 8."
    )]
    jobs: usize,
}

fn default_range() -> String {
    "~".into()
}

fn default_out() -> String {
    "/mnt/c/Users/Administrator/Downloads".into()
}

fn default_jobs() -> usize {
    8
}

fn default_template() -> String {
    "{user_id}/{post_id}_{index}".into()
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
        let range = range.parse::<Range>()?;
        let split = link.split("/").collect::<Vec<_>>();
        let (platform, user_id) = if split.len() == 2 {
            (split[0], split[1])
        } else {
            let Some(index) = split.iter().copied().position(|s| s == "user") else {
                return Err(anyhow!("Cannot parse link `{link}`"));
            };
            if index >= split.len() {
                return Err(anyhow!("Cannot parse link `{link}`"));
            }
            (split[index - 1], split[index + 1])
        };
        let platform = platform.to_string();

        let settings = Args {
            platform: platform.leak(),
            template: template.leak(),
            out: out.leak(),
            user_id: user_id.parse()?,
            range,
            cover,
            jobs,
        };
        Ok(settings)
    }
}
