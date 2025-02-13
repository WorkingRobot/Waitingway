use super::CronJob;
use crate::{await_cancellable, stopwatch::Stopwatch, storage::db};
use serenity::async_trait;
use sqlx::PgPool;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
        {
            let _s = Stopwatch::new("queue_estimates");
            await_cancellable!(db::login::refresh_queue_estimates(pool), stop_signal);
        }
        {
            let _s = Stopwatch::new("world_summaries");
            await_cancellable!(db::summary::refresh_world_summaries(pool), stop_signal);
        }
        Ok(())
    }
}
