use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Conf {
    pub out: Option<PathBuf>,
    pub template: Option<String>,
    pub jobs: Option<u8>,
}

impl Conf {
    pub async fn load() -> anyhow::Result<Self> {
        let conf_path = dirs_next::data_dir()
            .ok_or(anyhow::anyhow!("Can not locate conf path."))?
            .join("yaks")
            .join("conf.toml");
        let conf_str = tokio::fs::read_to_string(conf_path).await?;
        let conf = toml::from_str(&conf_str)?;
        Ok(conf)
    }
}
