use anyhow::bail;
use serenity::async_trait;
use sqlx::PgPool;
use std::{collections::HashMap, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

use crate::{
    await_cancellable,
    config::StasisConfig,
    db,
    models::{DCTravelResponse, DCTravelWorldInfo},
};

use super::CronJob;

pub struct RefreshTravelStates {
    config: StasisConfig,
    pool: PgPool,
    connector_path: std::path::PathBuf,
}

impl RefreshTravelStates {
    pub fn new(config: StasisConfig, pool: PgPool) -> std::io::Result<Self> {
        let connector_path = std::env::current_exe()?.with_file_name(format!(
            "TemporalStasis.Connector{}",
            std::env::consts::EXE_SUFFIX,
        ));
        if !connector_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Connector not found: {:?}", connector_path),
            ));
        }
        Ok(Self {
            config,
            pool,
            connector_path,
        })
    }
}

#[async_trait]
impl CronJob for RefreshTravelStates {
    const NAME: &'static str = "referesh_travel_states";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        let mut cmd = Command::new(self.connector_path.as_os_str())
            .args(&self.config.lobby_hosts)
            .args(["--version-file", &self.config.version_file])
            .args(["-u", &self.config.username])
            .args(["-p", &self.config.password])
            .args(["--uid-cache", &self.config.uid_cache.path])
            .args(["--uid-ttl", &self.config.uid_cache.ttl.to_string()])
            .args(["--dc-token-cache", &self.config.dc_token_cache.path])
            .args([
                "--dc-token-ttl",
                &self.config.dc_token_cache.ttl.to_string(),
            ])
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = cmd.stdout.take().unwrap();
        let status = await_cancellable!(cmd.wait(), stop_signal, {
            cmd.kill().await?;
        });
        drop(cmd);

        if !status.success() {
            bail!("non-zero exit code: {}", status);
        }

        let mut out = BufReader::new(stdout).lines();
        let mut travel_map: HashMap<u16, DCTravelWorldInfo> = HashMap::new();
        let mut travel_time: Option<i32> = None;
        loop {
            let line = match out.next_line().await? {
                None => break,
                Some(line) => line,
            };

            let line = serde_json::from_str::<DCTravelResponse>(&line)?;

            if let Some(error) = line.error {
                bail!(
                    "Response error: {} - {}; {} ({})",
                    error,
                    line.result.code,
                    line.result.errcode,
                    line.result.status
                );
            }

            let result = line.result;
            if result.code != "OK" {
                bail!(
                    "Response code: {}; {} ({})",
                    result.code,
                    result.errcode,
                    result.status
                );
            }

            if result.data.is_none() {
                bail!(
                    "No data: {}; {} ({})",
                    result.code,
                    result.errcode,
                    result.status
                );
            }

            let data = result.data.unwrap();
            for dc in data.datacenters {
                for world in &dc.worlds {
                    if let Some(w) = travel_map.get(&world.id) {
                        if *w == *world {
                            continue;
                        }
                        log::error!("World {} changed", w.id);
                        log::error!("Home {}: {:?}", data.home_world_id, dc.worlds.clone());
                        log::error!("Old: {:?}", w);
                    } else {
                        travel_map.insert(world.id, world.clone());
                    }
                }
            }

            travel_time = Some(data.average_elapsed_time);
        }

        if travel_map.is_empty() || travel_time.is_none() {
            bail!("No data");
        }

        log::info!("Travel time: {:?} sec", travel_time.unwrap());
        log::info!(
            "Travel prohibited worlds: {:?}",
            travel_map
                .iter()
                .filter(|w| w.1.prohibit != 0)
                .map(|w| w.0)
                .collect::<Vec<_>>()
        );

        db::add_travel_states(
            &self.pool,
            travel_map.into_values().collect(),
            travel_time.unwrap(),
        )
        .await?;
        Ok(())
    }
}
