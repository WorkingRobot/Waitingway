use super::CronJob;
use crate::{
    await_cancellable,
    config::StasisConfig,
    models::travel::{DCTravelResponse, DCTravelWorldInfo},
    storage::{db, game::worlds},
    subscriptions::{EndpointPublish, SubscriptionManager},
};
use anyhow::bail;
use itertools::Itertools;
use serenity::async_trait;
use sqlx::PgPool;
use std::{
    collections::{HashMap, HashSet},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub struct RefreshTravelStates {
    config: StasisConfig,
    pool: PgPool,
    subscriptions: SubscriptionManager,
    connector_path: std::path::PathBuf,
}

impl RefreshTravelStates {
    pub fn new(
        config: StasisConfig,
        pool: PgPool,
        subscriptions: SubscriptionManager,
    ) -> std::io::Result<Self> {
        let connector_path = std::env::current_exe()?.with_file_name(format!(
            "TemporalStasis.Connector{}",
            std::env::consts::EXE_SUFFIX,
        ));
        if !connector_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Connector not found: {connector_path:?}"),
            ));
        }
        Ok(Self {
            config,
            pool,
            subscriptions,
            connector_path,
        })
    }
}

#[async_trait]
impl CronJob for RefreshTravelStates {
    const NAME: &'static str = "refresh_travel_states";
    const PERIOD: Duration = Duration::from_secs(15);

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        let mut cmd = Command::new(self.connector_path.as_os_str());

        cmd.args(&self.config.lobby_hosts)
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
            .stderr(Stdio::piped());

        log::info!(
            "Running: {}",
            format!("{:?}", cmd.as_std()).replace(self.config.password.as_str(), "***")
        );

        let mut cmd = cmd.spawn()?;
        let mut stdout = cmd.stdout.take().unwrap();
        let mut stderr = cmd.stderr.take().unwrap();
        let status = await_cancellable!(cmd.wait(), stop_signal, {
            log::error!("Killing connector process...");
            let kill_err = cmd.kill().await;
            log::warn!("Stdout:");
            let mut out_buf = String::new();
            if let Err(e) = stdout.read_to_string(&mut out_buf).await {
                log::error!("Failed to read stdout: {}", e);
            } else {
                log::warn!("{out_buf}");
            }
            log::warn!("Stderr:");
            let mut err_buf = String::new();
            if let Err(e) = stderr.read_to_string(&mut err_buf).await {
                log::error!("Failed to read stderr: {}", e);
            } else {
                log::warn!("{err_buf}");
            }
            kill_err?;
        });
        log::info!("Connector process exited with: {}", status);
        drop(cmd);

        if !status.success() {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();

            if let Err(e) = stdout.read_to_string(&mut stdout_buf).await {
                log::error!("Failed to read stdout: {}", e);
                stdout_buf = "<failed to read stdout>".to_string();
            }

            if let Err(e) = stderr.read_to_string(&mut stderr_buf).await {
                log::error!("Failed to read stderr: {}", e);
                stderr_buf = "<failed to read stderr>".to_string();
            }

            bail!(
                "non-zero exit code: {}\nstdout:\n{}\nstderr:\n{}",
                status,
                stdout_buf,
                stderr_buf
            );
        }

        log::info!("Getting travel parameters...");
        let travel_params = worlds::get_data();
        let mut out = BufReader::new(stdout).lines();
        let mut travel_map: HashMap<u16, DCTravelWorldInfo> = HashMap::new();
        let mut travel_time: Option<i32> = None;
        log::info!("Parsing connector output...");
        loop {
            let line = match out.next_line().await? {
                None => break,
                Some(line) => line,
            };

            if let Some(line) = line.strip_prefix("[ERROR] ") {
                log::error!("{}", line);
                continue;
            }
            if let Some(line) = line.strip_prefix("[WARN] ") {
                log::warn!("{}", line);
                continue;
            }
            if let Some(line) = line.strip_prefix("[INFO] ") {
                log::info!("{}", line);
                continue;
            }
            if let Some(line) = line.strip_prefix("[VERBOSE] ") {
                log::trace!("{}", line);
                continue;
            }
            if let Some(line) = line.strip_prefix("[DEBUG] ") {
                log::debug!("{}", line);
                continue;
            }

            let line = match line.strip_prefix("[OUTPUT] ") {
                None => {
                    log::error!("Unexpected line: {}", line);
                    continue;
                }
                Some(line) => line,
            };

            let line = serde_json::from_str::<DCTravelResponse>(line)?;

            if let Some(error) = line.error {
                if line.result.errcode == "300" {
                    // PROHIBIT: All travel is prohibited
                    for w in &travel_params.worlds {
                        travel_map.entry(w.id).or_insert_with(|| DCTravelWorldInfo {
                            id: w.id,
                            travel: 0,
                            accept: 0,
                            prohibit: 1,
                        });
                    }
                    travel_time = Some(travel_time.unwrap_or_default());
                    continue;
                } else {
                    bail!(
                        "Response error: {} - {}; {} ({})",
                        error,
                        line.result.code,
                        line.result.errcode,
                        line.result.status
                    );
                }
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
                .sorted_unstable()
                .collect::<Vec<_>>()
        );

        let travel_states: Vec<DCTravelWorldInfo> = travel_map.values().cloned().collect();

        db::travel::add_travel_states(&self.pool, travel_states.clone(), travel_time.unwrap())
            .await?;

        let mut published_datacenters = HashSet::new();
        for world in &travel_states {
            if world.prohibit == 0 {
                if let Some(world_param) = travel_params.get_world_by_id(world.id) {
                    if published_datacenters.insert(world_param.datacenter.id) {
                        self.subscriptions
                            .publish_endpoint(EndpointPublish::Datacenter {
                                id: world_param.datacenter.id,
                                data: &world_param.datacenter,
                                worlds: travel_params
                                    .worlds
                                    .iter()
                                    .filter(|w| w.datacenter.id == world_param.datacenter.id)
                                    .map(|w| (w, travel_map.get(&w.id).unwrap().prohibit != 0))
                                    .collect::<Vec<_>>(),
                            })
                            .await?;
                    }
                    self.subscriptions
                        .publish_endpoint(EndpointPublish::World {
                            id: world.id,
                            data: world_param,
                        })
                        .await?;
                }
            }
        }

        Ok(())
    }
}
