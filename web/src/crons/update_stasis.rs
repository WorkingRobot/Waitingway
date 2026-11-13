use super::CronJob;
use crate::{await_cancellable, config::StasisConfig};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const STASIS_URL: &str =
    "https://raw.githubusercontent.com/WorkingRobot/ffxiv-patches/refs/heads/main/stasis.json";

pub struct UpdateStasis {
    client: Client,
    config: StasisConfig,
}

impl UpdateStasis {
    pub fn new(client: Client, config: StasisConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait]
impl CronJob for UpdateStasis {
    const NAME: &'static str = "update_stasis";
    const PERIOD: Duration = Duration::from_secs(60 * 5);

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        let resp = await_cancellable!(self.client.get(STASIS_URL).send(), stop_signal);
        let info = await_cancellable!(resp.error_for_status()?.json::<StasisInfo>(), stop_signal);

        std::fs::write(
            &self.config.version_file,
            serde_json::to_string_pretty(&info)?,
        )?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StasisInfo {
    pub blowfish_phrase: String,
    pub blowfish_version: u32,
    pub login_version: u16,
    pub boot_version: String,
    pub game_version: String,
    pub ex_versions: Vec<String>,

    pub game_exe: FileReport,
    pub boot_hashes: Vec<FileReport>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FileReport {
    pub file_name: String,
    pub file_size: u64,
    pub sha1_hash: String,
}
