use super::{
    commands::command_list,
    travel_param,
    utils::{COLOR_ERROR, COLOR_IN_QUEUE, COLOR_SUCCESS},
};
use crate::{
    config::DiscordConfig, db, discord::utils::format_duration, subscriptions::SubscriptionManager,
};
use rand::seq::SliceRandom;
use serenity::{
    all::{
        ActivityData, ChannelId, Context, CreateEmbed, CreateEmbedFooter, CreateMessage,
        DiscordJsonError, EditMessage, ErrorResponse, EventHandler, FormattedTimestamp,
        FormattedTimestampStyle, GatewayIntents, Http, HttpError, Member, Message, MessageId,
        ShardManager, Timestamp, UserId,
    },
    async_trait, Client,
};
use sqlx::PgPool;
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
    db: PgPool,
    subscriptions: OnceLock<SubscriptionManager>,
    client: OnceLock<RwLock<Client>>,
    http: OnceLock<Arc<Http>>,
    shards: OnceLock<Arc<ShardManager>>,
    current_activity: RwLock<Option<ActivityData>>,
    activity_handle: Mutex<OnceLock<JoinHandle<()>>>,
}

impl DiscordClient {
    pub async fn new(config: DiscordConfig, db: PgPool) -> Self {
        let intents = GatewayIntents::non_privileged() | GatewayIntents::GUILD_MEMBERS;

        travel_param::init_travel_params(&db)
            .await
            .expect("Error initializing travel params");

        let ret = Self {
            imp: Arc::new(DiscordClientImp {
                config,
                db,
                subscriptions: OnceLock::new(),
                client: OnceLock::new(),
                http: OnceLock::new(),
                shards: OnceLock::new(),
                current_activity: RwLock::new(None),
                activity_handle: Mutex::new(OnceLock::new()),
            }),
        };

        ret.imp
            .subscriptions
            .set(SubscriptionManager::new(ret.clone()))
            .unwrap_or_else(|_| unreachable!());

        let framework_client = ret.clone();
        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: command_list(),
                ..Default::default()
            })
            .setup(|ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(framework_client)
                })
            })
            .build();

        ret.imp
            .client
            .set(RwLock::new(
                Client::builder(&ret.imp.config.bot_token, intents)
                    .event_handler(ret.clone())
                    .framework(framework)
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
                    client.set_activity(Some(next_activity)).await;
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

    pub fn http(&self) -> &Http {
        self.imp.http.get().unwrap()
    }

    pub fn db(&self) -> &PgPool {
        &self.imp.db
    }

    pub fn subscriptions(&self) -> &SubscriptionManager {
        self.imp.subscriptions.get().unwrap()
    }

    pub fn config(&self) -> &DiscordConfig {
        &self.imp.config
    }

    async fn send_log_message(&self, message: impl Into<String>) -> Result<(), serenity::Error> {
        self.config()
            .log_channel_id
            .say(self.http(), message)
            .await?;
        Ok(())
    }

    async fn set_activity(&self, activity: Option<ActivityData>) {
        let runners = self.imp.shards.get().unwrap().runners.lock().await;

        runners.iter().for_each(|(_, runner)| {
            runner.runner_tx.set_activity(activity.clone());
        });
    }

    pub async fn onboard_user(&self, user_id: UserId) -> Result<Message, serenity::Error> {
        log::info!("Onboarding user {}", user_id);

        let already_in_guild = match self.config().guild_id.member(self.http(), user_id).await {
            Ok(_) => true,
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                error: DiscordJsonError { code: 10007, .. }, // Unknown Member
                ..
            }))) => false,
            Err(e) => return Err(e),
        };

        let channel = user_id.create_dm_channel(self.http()).await?;

        let invite_url = format!("https://discord.gg/{}", self.config().guild_invite_code);

        let embed = CreateEmbed::new().title("You're linked up!")
            .description(format!("You'll now get DMs from me whenever your queue is over {}.\n\n{}",
                self.config().queue_size_dm_threshold,
                if already_in_guild {
                    format!("Thanks for joining the [official Discord server]({})! It's the best way to stay up to date with Waitingway!", invite_url)
                }
                else {
                    format!("If you'd like to stay up to date with Waitingway, be sure to join the [official Discord server]({}).", invite_url)
                }))
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(COLOR_SUCCESS);

        let message = channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;
        if !already_in_guild {
            channel
                .send_message(self.http(), CreateMessage::new().content(invite_url))
                .await?;
        }
        Ok(message)
    }

    pub async fn offboard_user(&self, user_id: UserId) -> Result<(), serenity::Error> {
        let channel = user_id.create_dm_channel(self.http()).await?;

        let embed = CreateEmbed::new().title("You've been disconnected!")
            .description("This discord account will now no longer receive queue notifications from me!\n\nNote: You'll still receive notifications for queues from other computers.")
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(COLOR_ERROR);

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;

        Ok(())
    }

    pub async fn mark_user_connected(&self, user_id: UserId) -> Result<(), serenity::Error> {
        self.http()
            .add_member_role(
                self.config().guild_id,
                user_id,
                self.config().connected_role_id,
                Some("User is Connected"),
            )
            .await
    }

    pub async fn mark_user_disconnected(&self, user_id: UserId) -> Result<(), serenity::Error> {
        self.http()
            .remove_member_role(
                self.config().guild_id,
                user_id,
                self.config().connected_role_id,
                Some("User is Disconnected"),
            )
            .await
    }

    pub async fn send_queue_position(
        &self,
        user_id: UserId,
        character_name: &str,
        position: u32,
        now: time::OffsetDateTime,
        estimated: time::OffsetDateTime,
    ) -> Result<Message, serenity::Error> {
        let channel = user_id.create_dm_channel(self.http()).await?;

        channel
            .send_message(
                self.http(),
                CreateMessage::new().embed(Self::create_queue_embed(
                    character_name,
                    position,
                    now,
                    estimated,
                )),
            )
            .await
    }

    pub async fn update_queue_position(
        &self,
        message_id: MessageId,
        channel_id: ChannelId,
        character_name: &str,
        position: u32,
        now: time::OffsetDateTime,
        estimated: time::OffsetDateTime,
    ) -> Result<(), serenity::Error> {
        channel_id
            .edit_message(
                self.http(),
                message_id,
                EditMessage::new().embed(Self::create_queue_embed(
                    character_name,
                    position,
                    now,
                    estimated,
                )),
            )
            .await?;
        Ok(())
    }

    pub async fn send_queue_completion(
        &self,
        message_id: MessageId,
        channel_id: ChannelId,
        character_name: &str,
        queue_start_size: u32,
        queue_end_size: u32,
        duration: Duration,
        error_message: Option<String>,
        error_code: Option<i32>,
        identify_timeout: Option<time::OffsetDateTime>,
        successful: bool,
    ) -> Result<(), serenity::Error> {
        let delete_client = self.clone();
        let delete_task = tokio::task::spawn(async move {
            channel_id
                .delete_message(delete_client.http(), message_id)
                .await
        });

        if successful {
            self.send_queue_completion_successful(
                channel_id,
                character_name,
                queue_start_size,
                duration,
            )
            .await?;
        } else {
            self.send_queue_completion_unsuccessful(
                channel_id,
                character_name,
                queue_start_size,
                queue_end_size,
                duration,
                error_message,
                error_code,
                identify_timeout,
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
        character_name: &str,
        queue_start_size: u32,
        duration: Duration,
    ) -> Result<(), serenity::Error> {
        let embed = CreateEmbed::new()
            .title("Queue completed!")
            .description(format!("{} has been logged in successfully! Thanks for using Waitingway!\n\nYour queue size was {}, which was completed in {}.", character_name, queue_start_size, format_duration(duration)))
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(COLOR_SUCCESS);

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;
        Ok(())
    }

    async fn send_queue_completion_unsuccessful(
        &self,
        channel: ChannelId,
        character_name: &str,
        queue_start_size: u32,
        queue_end_size: u32,
        duration: Duration,
        error_message: Option<String>,
        error_code: Option<i32>,
        identify_timeout: Option<time::OffsetDateTime>,
    ) -> Result<(), serenity::Error> {
        let mut description = if let Some(identify_timeout) = identify_timeout {
            let identify_timeout: Timestamp = identify_timeout.into();
            format!(
                    "{} left the queue prematurely. If you didn't mean to, try queueing again by {} ({}) to not lose your spot.\n",
                    character_name,
                    FormattedTimestamp::new(identify_timeout, Some(FormattedTimestampStyle::LongTime)),
                    FormattedTimestamp::new(identify_timeout, Some(FormattedTimestampStyle::RelativeTime)),
                )
        } else {
            format!(
                "{} left the queue prematurely. If you didn't mean to, try queueing again.\n",
                character_name
            )
        };
        if let Some(error_message) = error_message {
            if let Some(error_code) = error_code {
                description
                    .push_str(format!("Error: {} ({})\n", error_message, error_code).as_str());
            }
        }
        description.push('\n');
        description.push_str(
            if queue_start_size == queue_end_size {
                format!(
                    "Your queue size was {}, and you were in queue for {}.",
                    queue_start_size,
                    format_duration(duration)
                )
            } else {
                format!(
                    "Your queue size started at {} and ended at {}, and you were in queue for {}.",
                    queue_start_size,
                    queue_end_size,
                    format_duration(duration)
                )
            }
            .as_str(),
        );
        let embed = CreateEmbed::new()
            .title("Unsuccessful Queue")
            .description(description)
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(COLOR_ERROR);

        channel
            .send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;
        Ok(())
    }

    fn create_queue_embed(
        character_name: &str,
        position: u32,
        now: time::OffsetDateTime,
        estimated: time::OffsetDateTime,
    ) -> CreateEmbed {
        let estimated: Timestamp = estimated.into();
        CreateEmbed::new()
            .title(format!("{}'s Queue", character_name))
            .description(format!(
                "You're in position {}. You'll login {} (at {})\n\nYou'll receive a DM from me when your queue completes.",
                position,
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::RelativeTime)),
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::LongTime)),
            ))
            .footer(CreateEmbedFooter::new("Last updated"))
            .timestamp(now)
            .color(COLOR_IN_QUEUE)
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
            ctx.set_activity(Some(activity.clone()));
        }
    }

    async fn guild_member_addition(&self, _ctx: Context, member: Member) {
        let is_connected = match db::does_connection_id_exist(self.db(), member.user.id.get()).await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Error checking if user is connected: {:?}", e);
                return;
            }
        };
        if is_connected {
            match self.mark_user_connected(member.user.id).await {
                Ok(_) => log::info!("Marked user {} as connected", member.user.id),
                Err(e) => log::error!(
                    "Error marking user {} as connected: {:?}",
                    member.user.id,
                    e
                ),
            }
        }
    }
}
