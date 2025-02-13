use rand::prelude::IndexedRandom;
use serenity::{all::ActivityData, async_trait};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::discord::DiscordClient;

use super::CronJob;

pub struct UpdateActivity {
    client: DiscordClient,
    activities: Vec<ActivityData>,
}

impl UpdateActivity {
    pub fn new(client: DiscordClient, activities: Vec<ActivityData>) -> Self {
        Self { client, activities }
    }
}

#[async_trait]
impl CronJob for UpdateActivity {
    const NAME: &'static str = "update_activity";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, _stop_signal: CancellationToken) -> anyhow::Result<()> {
        let activity = self.activities.choose(&mut rand::rng());
        self.client.set_activity(activity.cloned()).await;
        Ok(())
    }
}
