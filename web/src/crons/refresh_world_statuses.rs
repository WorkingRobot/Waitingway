use reqwest::Client;
use serenity::async_trait;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::{await_cancellable, db, models::WorldStatusResponse};

use super::CronJob;

pub struct RefreshWorldStatuses {
    client: Client,
    pool: PgPool,
}

impl RefreshWorldStatuses {
    pub fn new(client: Client, pool: PgPool) -> Self {
        Self { client, pool }
    }
}

#[async_trait]
impl CronJob for RefreshWorldStatuses {
    const NAME: &'static str = "refresh_world_statuses";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        let resp = await_cancellable!(
            self.client
                .get("https://frontier.ffxiv.com/v2/world/status.json")
                .send(),
            stop_signal
        );
        let resp = await_cancellable!(resp.json::<WorldStatusResponse>(), stop_signal);

        let worlds = resp
            .data
            .into_iter()
            .flat_map(|region| region.dc)
            .flat_map(|dc| dc.world);

        let result = db::add_world_statuses(&self.pool, worlds.collect()).await;
        if let Err(e) = result {
            log::error!("Failed to refresh world statuses: {:?}", e);
        }

        Ok(())
    }
}
