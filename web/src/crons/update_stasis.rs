use super::CronJob;
use crate::{await_cancellable, config::StasisConfig, stopwatch::Stopwatch};
use base64::Engine;
use futures_util::{StreamExt, TryStreamExt, stream};
use itertools::Itertools;
use serde::Serialize;
use serenity::async_trait;
use sha1::Digest;
use std::{collections::HashMap, sync::Mutex, time::Duration, u64};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;
use xiv_cache::{
    builder::ServerBuilder,
    file::CacheFile,
    server::{Server, SlugData},
};
use xiv_core::file::{slug::Slug, version::GameVersion};

pub struct UpdateStasis {
    cache_server: Server,

    config: StasisConfig,

    // (Version, Path) -> (File Size, Sha Hash)
    hash_cache: Mutex<HashMap<(GameVersion, String), (u64, Vec<u8>)>>,
}

impl UpdateStasis {
    pub async fn new(config: StasisConfig) -> anyhow::Result<Self> {
        let server = ServerBuilder::default()
            .clut_ram_capacity(32)
            .slug_update_interval_secs(u64::MAX) // Disable internal slug updates
            .ram_entry_capacity(1024 * 1024) // 1 million entries
            .storage_capacity_bytes(
                1024 * 1024, // 1 GiB
            );

        Ok(Self {
            cache_server: server.build().await?,
            config,
            hash_cache: Mutex::new(HashMap::new()),
        })
    }
}

const REPO_PREFIX: &str = "ffxivneo/win32/release";

#[async_trait]
impl CronJob for UpdateStasis {
    const NAME: &'static str = "update_stasis";
    const PERIOD: Duration = Duration::from_secs(60 * 30); // 30 minutes

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        await_cancellable!(self.cache_server.update_slugs(), stop_signal);

        log::info!("Updated slugs from remote");

        let slugs = await_cancellable!(self.cache_server.get_slug_list(), stop_signal);

        log::info!("Fetched slugs: {slugs:?}");

        let slugs = await_cancellable!(
            stream::iter(slugs)
                .map(async |slug| self.cache_server.get_slug(slug).await.map(|s| (slug, s)))
                .buffer_unordered(8)
                .try_collect::<Vec<_>>(),
            stop_signal
        );
        let boot_repo = slugs
            .iter()
            .find(|(_, s)| s.repository == format!("{REPO_PREFIX}/boot"))
            .ok_or_else(|| anyhow::anyhow!("Could not find boot repo slug"))?;
        let game_repo = slugs
            .iter()
            .find(|(_, s)| s.repository == format!("{REPO_PREFIX}/game"))
            .ok_or_else(|| anyhow::anyhow!("Could not find game repo slug"))?;
        let ex_repos = slugs
            .iter()
            .filter(|(_, s)| s.repository.starts_with(&format!("{REPO_PREFIX}/ex")))
            .sorted_by_key(|(_, s)| &s.repository)
            .collect_vec();

        log::info!(
            "Updating stasis info: boot {}, game {}, ex {:?}",
            boot_repo.1.latest_version,
            game_repo.1.latest_version,
            ex_repos
                .iter()
                .map(|(_, s)| s.latest_version.to_string())
                .collect::<Vec<_>>(),
        );

        let game_exe = await_cancellable!(
            self.fetch_file_report(game_repo, "ffxiv_dx11.exe"),
            stop_signal
        );

        let mut boot_hashes = vec![];
        for name in [
            "ffxivboot.exe",
            "ffxivboot64.exe",
            "ffxivlauncher64.exe",
            "ffxivupdater64.exe",
        ] {
            let report = await_cancellable!(self.fetch_file_report(boot_repo, name), stop_signal);
            boot_hashes.push(report);
        }

        let info = StasisInfo {
            blowfish_phrase: self.config.blowfish_phrase.clone(),
            blowfish_version: self.config.blowfish_version,
            login_version: self.config.login_version,
            boot_version: boot_repo.1.latest_version.to_string(),
            game_version: game_repo.1.latest_version.to_string(),
            ex_versions: ex_repos
                .iter()
                .map(|(_, s)| s.latest_version.to_string())
                .collect(),
            game_exe,
            boot_hashes,
        };

        let info_json = serde_json::to_string_pretty(&info)?;

        std::fs::write(&self.config.version_file, info_json)?;

        Ok(())
    }
}

impl UpdateStasis {
    async fn fetch_file_info(
        &self,
        slug: Slug,
        version: GameVersion,
        file: String,
    ) -> anyhow::Result<(u64, Vec<u8>)> {
        let cache_key = (version.clone(), file.clone());
        {
            let cache_guard = self.hash_cache.lock().unwrap();
            if let Some(cached) = cache_guard.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let file = CacheFile::new(self.cache_server.clone(), slug, version, file).await?;
        let file_size = file.len();

        let mut file_stream = file.into_reader();

        let mut file_data = vec![0u8; file_size as usize];
        file_stream.read_exact(&mut file_data).await?;

        let mut hasher = sha1::Sha1::new();
        hasher.update(&file_data);
        let hash_result = hasher.finalize().to_vec();

        {
            let mut cache_guard = self.hash_cache.lock().unwrap();
            cache_guard.insert(cache_key, (file_size, hash_result.clone()));
        }

        Ok((file_size, hash_result))
    }

    async fn fetch_file_report(
        &self,
        slug_info: &(Slug, SlugData),
        file: impl Into<String>,
    ) -> anyhow::Result<FileReport> {
        let file = file.into();
        let _s = Stopwatch::new(format!("{}/{file}", slug_info.1.repository));
        let (file_size, sha1_hash) = self
            .fetch_file_info(
                slug_info.0,
                slug_info.1.latest_version.clone(),
                file.clone(),
            )
            .await?;
        Ok(FileReport::new(file, file_size, &sha1_hash))
    }
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FileReport {
    pub file_name: String,
    pub file_size: u64,
    pub sha1_hash: String,
}

impl FileReport {
    pub fn new(file_name: String, file_size: u64, sha1_hash: &[u8]) -> Self {
        Self {
            file_name,
            file_size,
            sha1_hash: base64::prelude::BASE64_STANDARD.encode(sha1_hash),
        }
    }
}
