use super::{
    commands::command_list,
    travel_param,
    utils::{format_duration, COLOR_ERROR, COLOR_IN_QUEUE, COLOR_SUCCESS},
};
use crate::{
    config::DiscordConfig, db, discord::utils::format_queue_duration,
    subscriptions::SubscriptionManager,
};
use futures_util::future::try_join_all;
use itertools::Itertools;
use serenity::{
    all::{
        ActionRowComponent, ActivityData, ChannelId, ComponentInteractionDataKind, Context,
        CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
        CreateInteractionResponseMessage, CreateMessage, DiscordJsonError, EditMessage,
        ErrorResponse, EventHandler, FormattedTimestamp, FormattedTimestampStyle, GatewayIntents,
        Http, HttpError, Interaction, Member, Mentionable, Message, MessageId, RoleId,
        ShardManager, Timestamp, UserId,
    },
    async_trait, Client,
};
use sqlx::PgPool;
use std::sync::{Arc, OnceLock};
use time::{Duration, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct DiscordClient {
    imp: Arc<DiscordClientImp>,
}

impl std::fmt::Debug for DiscordClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordClient")
            .field("name", &*self.client_blocking().cache.current_user())
            .finish()
    }
}

struct DiscordClientImp {
    config: DiscordConfig,
    db: PgPool,
    subscriptions: OnceLock<SubscriptionManager>,
    client: OnceLock<RwLock<Client>>,
    http: OnceLock<Arc<Http>>,
    shards: OnceLock<Arc<ShardManager>>,
    current_activity: RwLock<Option<ActivityData>>,
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
            }),
        };

        let framework_client = ret.clone();
        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: command_list(),
                on_error: |error| {
                    Box::pin(async move {
                        log::error!("Error in command: {:?}", error);
                    })
                },
                ..Default::default()
            })
            .setup(|ctx, _ready, _framework| {
                Box::pin(async move {
                    let (global_commands, internal_commands): (Vec<_>, Vec<_>) = command_list()
                        .into_iter()
                        .partition(|c| !c.identifying_name.starts_with("internal"));
                    log::info!("Registering global commands: {global_commands:?}");
                    poise::builtins::register_globally(ctx, &global_commands).await?;
                    log::info!("Registering internal guild commands: {internal_commands:?}");
                    poise::builtins::register_in_guild(
                        ctx,
                        &internal_commands,
                        framework_client.config().guild_id,
                    )
                    .await?;
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
        self.client_mut().await.start_autosharded().await
    }

    pub async fn stop(&self) {
        if let Err(e) = self.send_log_message("Shutting down!").await {
            log::error!("Error sending log message: {:?}", e);
        }

        self.imp.shards.get().unwrap().shutdown_all().await;
    }

    fn client_blocking(&self) -> RwLockReadGuard<Client> {
        self.imp.client.get().unwrap().blocking_read()
    }

    async fn client(&self) -> RwLockReadGuard<Client> {
        self.imp.client.get().unwrap().read().await
    }

    async fn client_mut(&self) -> RwLockWriteGuard<Client> {
        self.imp.client.get().unwrap().write().await
    }

    pub fn shards(&self) -> &ShardManager {
        self.imp.shards.get().unwrap()
    }

    pub fn http(&self) -> &Http {
        self.imp.http.get().unwrap()
    }

    pub fn db(&self) -> &PgPool {
        &self.imp.db
    }

    pub fn set_subscriptions(&self, subscriptions: SubscriptionManager) {
        self.imp
            .subscriptions
            .set(subscriptions)
            .map_err(|_| ())
            .expect("Subscriptions already set");
    }

    pub fn subscriptions(&self) -> &SubscriptionManager {
        self.imp.subscriptions.get().expect("Subscriptions not set")
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

    pub async fn set_activity(&self, activity: Option<ActivityData>) {
        let mut current_activity = self.imp.current_activity.write().await;
        match (&activity, &*current_activity) {
            (Some(a), Some(b)) if a.kind == b.kind && a.name == b.name => return,
            _ => {}
        };
        current_activity.clone_from(&activity);
        self.broadcast_activity(activity).await;
    }

    async fn broadcast_activity(&self, activity: Option<ActivityData>) {
        let runners = self.shards().runners.lock().await;

        runners.values().for_each(|runner| {
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
                    format!("Thanks for joining the [official Discord server]({invite_url})! It's the best way to stay up to date with Waitingway!")
                }
                else {
                    format!("If you'd like to stay up to date with Waitingway, be sure to join the [official Discord server]({invite_url}).")
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
            format!("{character_name} left the queue prematurely. If you didn't mean to, try queueing again.\n")
        };
        if let Some(error_message) = error_message {
            if let Some(error_code) = error_code {
                description.push_str(format!("Error: {error_message} ({error_code})\n").as_str());
            }
        }
        description.push('\n');
        description.push_str(
            if queue_start_size == queue_end_size {
                format!(
                    "Your queue size was {}, and you were in queue for {}.",
                    queue_start_size,
                    format_queue_duration(duration)
                )
            } else {
                format!(
                    "Your queue size started at {} and ended at {}, and you were in queue for {}.",
                    queue_start_size,
                    queue_end_size,
                    format_queue_duration(duration)
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
            .title(format!("{character_name}'s Queue"))
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

        if let Some(activity) = self.imp.current_activity.read().await.as_ref().cloned() {
            ctx.set_activity(Some(activity));
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
                Ok(()) => log::info!("Marked user {} as connected", member.user.id),
                Err(e) => log::error!(
                    "Error marking user {} as connected: {:?}",
                    member.user.id,
                    e
                ),
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Some(interaction) = interaction.into_message_component() else {
            return;
        };
        match interaction.guild_id {
            Some(id) if id == self.config().guild_id => id,
            _ => return,
        };
        if interaction.data.custom_id != "role_selector" {
            return;
        }
        if interaction.message.author.id != ctx.cache.current_user().id {
            return;
        }

        if let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
            let values = values
                .iter()
                .map(|v| v.parse::<u64>().ok().map(RoleId::new))
                .collect::<Option<Vec<_>>>();
            let Some(values) = values else { return };

            let Some(member) = &interaction.member else {
                return;
            };
            let options = interaction.message.components.first().and_then(|c| {
                c.components.iter().find_map(|r| match r {
                    ActionRowComponent::SelectMenu(menu) => menu
                        .options
                        .iter()
                        .map(|o| o.value.as_str().parse::<u64>().ok().map(RoleId::new))
                        .collect::<Option<Vec<_>>>(),
                    _ => None,
                })
            });
            let Some(options) = options else { return };
            let mut additions = vec![];
            let mut removals = vec![];
            for option in options {
                if values.contains(&option) {
                    // Add role if not already present
                    if !member.roles.contains(&option) {
                        additions.push(option);
                    }
                } else {
                    // Remove role if present
                    if member.roles.contains(&option) {
                        removals.push(option);
                    }
                }
            }

            let futures_a = additions.iter().map(|r| member.add_role(&ctx.http, r));
            let futures_b = removals.iter().map(|r| member.remove_role(&ctx.http, r));
            let futures = tokio::try_join!(try_join_all(futures_a), try_join_all(futures_b));
            match futures {
                Ok(_) => {
                    log::info!(
                        "Gave roles {:?} and removed roles {:?} to user {}",
                        additions,
                        removals,
                        interaction.user.id
                    );
                }
                Err(e) => {
                    log::error!("Error modifying roles: {:?}", e);
                    return;
                }
            }
            match interaction
                .create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .embed(
                                CreateEmbed::new()
                                    .title("Updated Roles")
                                    .description(
                                        additions
                                            .into_iter()
                                            .map(|r| format!("**+** {}", r.mention()))
                                            .chain(
                                                removals
                                                    .into_iter()
                                                    .map(|r| format!("**-** {}", r.mention())),
                                            )
                                            .join("\n"),
                                    )
                                    .color(COLOR_SUCCESS),
                            )
                            .ephemeral(true),
                    ),
                )
                .await
            {
                Ok(()) => {
                    log::info!("Gave roles {:?} to user {}", values, interaction.user.id);
                }
                Err(e) => {
                    log::error!("Error acknowledging interaction: {:?}", e);
                    return;
                }
            }
        }
    }
}
