pub mod refresh_queue_estimates;
pub use refresh_queue_estimates::RefreshQueueEstimates;

pub mod refresh_travel_states;
pub use refresh_travel_states::RefreshTravelStates;

use std::time::Duration;

use serenity::async_trait;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait CronJob {
    const NAME: &'static str;
    const PERIOD: Duration;

    async fn run(&self, stop_signal: CancellationToken);
}

pub fn create_cron_job<T: CronJob + Send + 'static>(job: T) -> CancellationToken {
    let stop_signal = CancellationToken::new();
    let signal = stop_signal.clone();
    tokio::spawn(async move {
        let new_job = job;
        loop {
            log::info!("Running cron job \"{}\"", T::NAME);
            new_job.run(signal.clone()).await;

            tokio::select! {
                _ = signal.cancelled() => {
                    log::info!("Cron job \"{}\" was cancelled", T::NAME);
                    break;
                }

                _ = tokio::time::sleep(T::PERIOD) => {}
            }
        }
    });
    stop_signal
}
