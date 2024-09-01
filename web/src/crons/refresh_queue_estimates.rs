use std::time::Duration;

use serenity::async_trait;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::db;

use super::CronJob;

pub struct RefreshQueueEstimates {
    pool: PgPool,
}

impl RefreshQueueEstimates {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CronJob for RefreshQueueEstimates {
    const NAME: &'static str = "refresh_queue_estimates";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, stop_signal: CancellationToken) {
        let pool = &self.pool;
        tokio::select! {
            result = db::refresh_queue_estimates(pool) => {
                if let Err(e) = result {
                    log::error!("Failed to refresh queue estimates: {}", e);
                }
            }
            _ = stop_signal.cancelled() => {}
        }
    }
}
