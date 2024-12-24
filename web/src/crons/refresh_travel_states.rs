use serenity::async_trait;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

use crate::{
    config::StasisConfig,
    db,
    models::{DCTravelResponse, DCTravelWorldInfo},
};

use super::CronJob;

pub struct RefreshTravelStates {
    config: StasisConfig,
    pool: PgPool,
}

impl RefreshTravelStates {
    pub fn new(config: StasisConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }
}

#[async_trait]
impl CronJob for RefreshTravelStates {
    const NAME: &'static str = "referesh_travel_states";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, stop_signal: CancellationToken) {
        let cmd = Command::new("./TemporalStasis.Connector")
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
            .spawn();
        let mut cmd = match cmd {
            Err(e) => {
                log::error!("Failed to start refresh travel states: {}", e);
                return;
            }
            Ok(cmd) => cmd,
        };
        let stdout = cmd.stdout.take().unwrap();
        let mut status: Option<std::io::Result<ExitStatus>> = None;
        tokio::select! {
            s = cmd.wait() => {status = Some(s);}
            _ = stop_signal.cancelled() => {
                if let Err(e) = cmd.kill().await {
                    log::error!("Failed to kill refresh travel states: {}", e);
                }
            }
        };
        drop(cmd);

        if stop_signal.is_cancelled() || status.is_none() {
            return;
        }

        match status.unwrap() {
            Err(e) => {
                log::error!("Failed to wait for refresh travel states: {}", e);
                return;
            }
            Ok(status) => {
                if !status.success() {
                    log::error!(
                        "Failed to refresh travel states: non-zero exit code ({})",
                        status
                    );
                    return;
                }
            }
        }

        let mut out = BufReader::new(stdout).lines();

        let mut travel_map: HashMap<u16, DCTravelWorldInfo> = HashMap::new();
        let mut travel_time: Option<i32> = None;
        loop {
            let line = match out.next_line().await {
                Err(e) => {
                    log::error!("Failed to read travel states line: {}", e);
                    return;
                }
                Ok(None) => {
                    break;
                }
                Ok(Some(line)) => line,
            };

            let line = match serde_json::from_str::<DCTravelResponse>(&line) {
                Err(e) => {
                    log::error!("Failed to parse refresh travel states line: {}", e);
                    continue;
                }
                Ok(line) => line,
            };

            if let Some(error) = line.error {
                log::error!(
                    "Failed to refresh travel states: {} - {}; {} ({})",
                    error,
                    line.result.code,
                    line.result.errcode,
                    line.result.status
                );
                continue;
            }

            let result = line.result;
            if result.code != "OK" {
                log::error!(
                    "Failed to refresh travel states: {}; {} ({})",
                    result.code,
                    result.errcode,
                    result.status
                );
                continue;
            }

            if result.data.is_none() {
                log::error!(
                    "Failed to refresh travel states: no data - {}; {} ({})",
                    result.code,
                    result.errcode,
                    result.status
                );
                continue;
            }

            let data = result.data.unwrap();
            for dc in data.datacenters {
                // This actually doesn't matter; prohibitFlag is always correct
                // if dc.dc == data.home_dc {
                //     continue;
                // }
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

            if let Some(t) = travel_time {
                if t != data.average_elapsed_time {
                    log::error!("Travel time changed");
                    log::error!("Home {}: {}", data.home_world_id, data.average_elapsed_time);
                    log::error!("Old: {}", t);
                }
            } else {
                travel_time = Some(data.average_elapsed_time);
            }
        }

        if travel_map.is_empty() || travel_time.is_none() {
            log::error!("Failed to refresh travel states: no data");
            return;
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

        if let Err(e) = db::add_travel_states(
            &self.pool,
            travel_map.into_values().collect(),
            travel_time.unwrap(),
        )
        .await
        {
            log::error!("Failed to add travel states: {}", e);
        }
    }
}
