pub mod refresh_materialized_views;
pub use refresh_materialized_views::RefreshMaterializedViews;

pub mod refresh_travel_states;
pub use refresh_travel_states::RefreshTravelStates;

pub mod refresh_world_statuses;
pub use refresh_world_statuses::RefreshWorldStatuses;

use std::time::Duration;

use serenity::async_trait;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait CronJob {
    const NAME: &'static str;
    const PERIOD: Duration;

    async fn run(&self, stop_signal: CancellationToken) -> anyhow::Result<()>;
}

pub fn create_cron_job<T: CronJob + Send + 'static>(job: T) -> CancellationToken {
    let stop_signal = CancellationToken::new();
    let signal = stop_signal.clone();
    tokio::spawn(async move {
        let new_job = job;
        loop {
            log::info!("Running cron job \"{}\"", T::NAME);
            let timer = tokio::time::Instant::now();
            let result = new_job.run(signal.clone()).await;
            log::info!("Cron job \"{}\" took {:?}", T::NAME, timer.elapsed());
            if signal.is_cancelled() {
                log::warn!("Cron job \"{}\" was cancelled", T::NAME);
            }
            if let Err(e) = result {
                log::error!("Cron job \"{}\" failed: {:?}", T::NAME, e);
            }

            tokio::select! {
                _ = signal.cancelled() => { break; }
                _ = tokio::time::sleep(T::PERIOD) => {}
            }
        }
    });
    stop_signal
}

#[macro_export]
macro_rules! await_cancellable {
    ($expr:expr, $signal:expr) => {
        await_cancellable!($expr, $signal, {})
    };

    ($expr:expr, $signal:expr, $cancelled:tt) => {{
        let result: anyhow::Result<Option<_>> = tokio::select! {
            s = $expr => Ok(Some(s?)),
            _ = $signal.cancelled() => {
                $cancelled;
                Ok(None)
            }
        };
        match result? {
            Some(s) => s,
            None => return Ok(()),
        }
    }};
}
