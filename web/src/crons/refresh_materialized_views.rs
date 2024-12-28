use std::time::Duration;

use serenity::async_trait;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::{await_cancellable, db};

use super::CronJob;

pub struct RefreshMaterializedViews {
    pool: PgPool,
}

impl RefreshMaterializedViews {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CronJob for RefreshMaterializedViews {
    const NAME: &'static str = "refresh_materialized_views";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()> {
        let pool = &self.pool;
        await_cancellable!(db::refresh_queue_estimates(pool), stop_signal);
        await_cancellable!(db::refresh_world_summaries(pool), stop_signal);
        Ok(())
    }
}
