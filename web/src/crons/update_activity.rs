use rand::prelude::IndexedRandom;
use serenity::{all::ActivityData, async_trait};
use std::time::Duration;
use thousands::Separable;
use tokio_util::sync::CancellationToken;

use crate::{
    config::DiscordActivity,
    discord::{
        utils::{command_invokes, install_stats, member_count},
        DiscordClient,
    },
};

use super::CronJob;

pub struct UpdateActivity {
    client: DiscordClient,
    activities: Vec<DiscordActivity>,
}

impl UpdateActivity {
    pub fn new(client: DiscordClient) -> Self {
        Self {
            activities: client.config().activities.clone(),
            client,
        }
    }

    async fn process(client: &DiscordClient, mut activity: DiscordActivity) -> ActivityData {
        if activity.text.contains("{servers}") || activity.text.contains("{users}") {
            let (servers, users) = install_stats(client).await;
            activity.text = activity
                .text
                .replace("{servers}", &servers.separate_with_commas());
            activity.text = activity
                .text
                .replace("{users}", &users.separate_with_commas());
        }
        if activity.text.contains("{members}") {
            let members = member_count(client).await;
            activity.text = activity
                .text
                .replace("{members}", &members.separate_with_commas());
        }
        if activity.text.contains("{commands}") {
            let commands = command_invokes(client).await;
            activity.text = activity
                .text
                .replace("{commands}", &commands.separate_with_commas());
        }
        activity.into()
    }
}

#[async_trait]
impl CronJob for UpdateActivity {
    const NAME: &'static str = "update_activity";
    const PERIOD: Duration = Duration::from_secs(60);

    async fn run(&self, _stop_signal: CancellationToken) -> anyhow::Result<()> {
        let activity = self.activities.choose(&mut rand::rng());
        let activity = match activity {
            Some(activity) => Some(Self::process(&self.client, activity.clone()).await),
            None => None,
        };
        self.client.set_activity(activity).await;
        Ok(())
    }
}
