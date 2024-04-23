use crate::config::DiscordConfig;
use rand::seq::SliceRandom;
use serenity::{
    all::{
        ActivityData, AddMember, ChannelId, Color, Context, CreateEmbed, CreateEmbedFooter,
        CreateMessage, EditMessage, EventHandler, FormattedTimestamp, FormattedTimestampStyle,
        GatewayIntents, Http, Message, MessageId, OnlineStatus, ShardManager, Timestamp, UserId,
    },
    async_trait, Client,
};
use std::sync::{Arc, OnceLock};
use time::{Duration, OffsetDateTime};
use tokio::{
    sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::JoinHandle,
};

#[derive(Clone)]
pub struct DiscordClient {
    imp: Arc<DiscordClientImp>,
}

struct DiscordClientImp {
    config: DiscordConfig,
    client: OnceLock<RwLock<Client>>,
    http: OnceLock<Arc<Http>>,
    shards: OnceLock<Arc<ShardManager>>,
    current_activity: RwLock<Option<ActivityData>>,
    activity_handle: Mutex<OnceLock<JoinHandle<()>>>,
}

impl DiscordClient {
    pub async fn new(config: DiscordConfig) -> Self {
        let intents = GatewayIntents::empty();

        let ret = Self {
            imp: Arc::new(DiscordClientImp {
                config,
                client: OnceLock::new(),
                http: OnceLock::new(),
                shards: OnceLock::new(),
                current_activity: RwLock::new(None),
                activity_handle: Mutex::new(OnceLock::new()),
            }),
        };

        ret.imp
            .client
            .set(RwLock::new(
                Client::builder(&ret.imp.config.bot_token, intents)
                    .event_handler(ret.clone())
                    .await
                    .expect("Error creating client"),
            ))
            .unwrap_or_else(|_| unreachable!());

        ret.imp
            .http
            .set(ret.client().await.http.clone())
            .unwrap_or_else(|_| unreachable!());

        ret.imp
            .shards
            .set(ret.client().await.shard_manager.clone())
            .unwrap_or_else(|_| unreachable!());

        ret
    }

    pub async fn start(&self) -> Result<(), serenity::Error> {
        let client = self.clone();
        let activity_handle = tokio::task::spawn(async move {
            let interval = std::time::Duration::from_secs(client.config().activity_update_interval);
            loop {
                let next_activity: ActivityData = client
                    .imp
                    .config
                    .activities
                    .choose(&mut rand::thread_rng())
                    .expect("No activities")
                    .clone()
                    .into();

                let mut current_activity = client.imp.current_activity.write().await;

                let should_modify = if let Some(current_activity) = current_activity.as_ref() {
                    current_activity.kind != next_activity.kind
                        || current_activity.name != next_activity.name
                } else {
                    true
                };

                if should_modify {
                    *current_activity = Some(next_activity.clone());
                }
                drop(current_activity);
                if should_modify {
                    client
                        .set_presence(Some(next_activity), OnlineStatus::Online)
                        .await;
                }
                tokio::time::sleep(interval).await;
            }
        });

        self.imp
            .activity_handle
            .lock()
            .await
            .set(activity_handle)
            .unwrap();

        self.client_mut().await.start_autosharded().await?;

        Ok(())
    }

    pub async fn stop(&self) {
        if let Err(e) = self.send_log_message("Shutting down!").await {
            log::error!("Error sending log message: {:?}", e);
        }

        if let Some(activity_handle) = self.imp.activity_handle.lock().await.take() {
            activity_handle.abort();
            activity_handle.await.unwrap_err();
        }

        self.imp.shards.get().unwrap().shutdown_all().await;
    }

    async fn client(&self) -> RwLockReadGuard<Client> {
        self.imp.client.get().unwrap().read().await
    }

    async fn client_mut(&self) -> RwLockWriteGuard<Client> {
        self.imp.client.get().unwrap().write().await
    }

    fn http(&self) -> &Http {
        &self.imp.http.get().unwrap()
    }

    fn config(&self) -> &DiscordConfig {
        &self.imp.config
    }

    async fn send_log_message(&self, message: impl Into<String>) -> Result<(), serenity::Error> {
        self.config()
            .log_channel_id
            .say(self.http(), message)
            .await?;
        Ok(())
    }

    async fn set_presence(&self, activity: Option<ActivityData>, status: OnlineStatus) {
        let runners = self.imp.shards.get().unwrap().runners.lock().await;

        runners.iter().for_each(|(_, runner)| {
            runner.runner_tx.set_presence(activity.clone(), status);
        });
    }

    pub async fn onboard_user(
        &self,
        user_id: UserId,
        access_token: String,
    ) -> Result<Message, serenity::Error> {
        let member = self
            .config()
            .guild_id
            .add_member(self.http(), user_id, AddMember::new(access_token))
            .await?;

        let has_already_joined = member.is_none();

        let channel = user_id.create_dm_channel(self.http()).await?;

        let embed = CreateEmbed::new().title("You're linked up!")
            .description(format!("You'll now get DMs from me whenever your queue is over {}.\n\n{}\n*Make sure you stay in the server to get notifications and enable DMs from server members.*",
                self.config().queue_size_dm_threshold,
                if has_already_joined {
                    "You've already joined the server, so you're all set!"
                }
                else {
                    "You've been added to the server, so you're all set!"
                }))
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(Color::from_rgb(16, 240, 12));

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await
    }

    pub async fn offboard_user(&self, user_id: UserId) -> Result<(), serenity::Error> {
        let channel = user_id.create_dm_channel(self.http()).await?;

        let embed = CreateEmbed::new().title("You've been disconnected!")
            .description("This discord account will now no longer receive queue notifications from me!\n\nNote: You'll still receive notifications for queues from other computers.")
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(Color::from_rgb(235, 96, 94));

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;

        Ok(())
    }

    pub async fn send_queue_position(
        &self,
        user_id: UserId,
        position: u32,
        now: time::PrimitiveDateTime,
        estimated: time::PrimitiveDateTime,
    ) -> Result<Message, serenity::Error> {
        let channel = user_id.create_dm_channel(self.http()).await?;

        channel
            .send_message(
                self.http(),
                CreateMessage::new().embed(Self::create_queue_embed(position, now, estimated)),
            )
            .await
    }

    pub async fn update_queue_position(
        &self,
        message_id: MessageId,
        channel_id: ChannelId,
        position: u32,
        now: time::PrimitiveDateTime,
        estimated: time::PrimitiveDateTime,
    ) -> Result<(), serenity::Error> {
        channel_id
            .edit_message(
                self.http(),
                message_id,
                EditMessage::new().embed(Self::create_queue_embed(position, now, estimated)),
            )
            .await?;
        Ok(())
    }

    pub async fn send_queue_completion(
        &self,
        message_id: MessageId,
        channel_id: ChannelId,
        queue_start_size: u32,
        queue_end_size: u32,
        duration: Duration,
        successful: bool,
    ) -> Result<(), serenity::Error> {
        let delete_client = self.clone();
        let delete_task = tokio::task::spawn(async move {
            channel_id
                .delete_message(delete_client.http(), message_id)
                .await
        });

        if successful {
            self.send_queue_completion_successful(channel_id, queue_start_size, duration)
                .await?;
        } else {
            self.send_queue_completion_unsuccessful(
                channel_id,
                queue_start_size,
                queue_end_size,
                duration,
            )
            .await?;
        }

        delete_task
            .await
            .map_err(|_| serenity::Error::Other("Failed to delete message"))??;

        Ok(())
    }

    async fn send_queue_completion_successful(
        &self,
        channel: ChannelId,
        queue_start_size: u32,
        duration: Duration,
    ) -> Result<(), serenity::Error> {
        let embed = CreateEmbed::new()
            .title("Queue completed!")
            .description(format!("You've been logged in successfully! Thanks for using Waitingway!\n\nYour queue size was {}, which was completed in {}.", queue_start_size, format_duration(duration)))
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(Color::from_rgb(16, 240, 12));

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;
        Ok(())
    }

    async fn send_queue_completion_unsuccessful(
        &self,
        channel: ChannelId,
        queue_start_size: u32,
        queue_end_size: u32,
        duration: Duration,
    ) -> Result<(), serenity::Error> {
        let embed = CreateEmbed::new()
            .title("Unsuccessful Queue")
            .description(
                if queue_start_size == queue_end_size {
                    format!("You left the queue prematurely. If you didn't mean to, try queueing again.\n\nYour queue size was {}, and you were in queue for {}.", queue_start_size, format_duration(duration))
                }
                else {
                    format!("You left the queue prematurely. If you didn't mean to, try queueing again.\n\nYour queue size started at {} and ended at {}, and you were in queue for {}.", queue_start_size, queue_end_size, format_duration(duration))
                }
            )
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(Color::from_rgb(235, 96, 94));

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;
        Ok(())
    }

    fn create_queue_embed(
        position: u32,
        now: time::PrimitiveDateTime,
        estimated: time::PrimitiveDateTime,
    ) -> CreateEmbed {
        let estimated: Timestamp = estimated.assume_utc().into();
        CreateEmbed::new()
            .title("Login Queue")
            .description(format!(
                "You're in position {}. You'll login {} (at {})\n\nYou'll receive a DM from me when your queue completes.",
                position,
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::RelativeTime)),
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::LongTime)),
            ))
            .footer(CreateEmbedFooter::new("Last updated"))
            .timestamp(now.assume_utc())
            .color(Color::BLITZ_BLUE)
    }
}

#[async_trait]
impl EventHandler for DiscordClient {
    async fn ready(&self, ctx: Context, data_about_bot: serenity::model::gateway::Ready) {
        let mut msg = "Started".to_string();
        if let Some(s) = data_about_bot.shard {
            msg = format!("Started shard {}", s.id);
        }
        if let Err(e) = self.send_log_message(msg).await {
            log::error!("Error sending log message: {:?}", e);
        }

        if let Some(activity) = self.imp.current_activity.read().await.as_ref() {
            ctx.set_presence(Some(activity.clone()), OnlineStatus::Online);
        }
    }
}

fn format_duration(duration: time::Duration) -> String {
    let seconds = duration.whole_seconds();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let days = hours / 24;
    let hours = hours % 24;

    if days > 0 {
        return format!("{}d {:02}:{:02}:{:02}", days, hours, minutes, seconds);
    } else if hours > 0 {
        return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    } else {
        return format!("{:02}:{:02}", minutes, seconds);
    }
}
