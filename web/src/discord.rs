use serenity::{
    all::{
        AddMember, ChannelId, Color, Context, CreateEmbed, CreateMessage, EventHandler,
        GatewayIntents, Http, ShardManager, UserId,
    },
    async_trait, Client,
};
use std::sync::{Arc, OnceLock};
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::config::DiscordConfig;

enum DiscordMessage {
    ConnectionMade(UserId),
    ConnectionLost(UserId),
    EnteredQueue(UserId),
    UpdatedQueue {
        user_id: UserId,
        position: i32,
        update_time: time::PrimitiveDateTime,
        estimated: time::PrimitiveDateTime,
    },
    LeftQueue {
        user_id: UserId,
        successful: bool,
    },
    Onboard {
        user_id: UserId,
        access_token: String,
    },
    Exit,
}

#[derive(Clone)]
pub struct DiscordClient {
    imp: Arc<DiscordClientImp>,
}

struct DiscordClientImp {
    config: DiscordConfig,
    client: OnceLock<RwLock<Client>>,
    http: OnceLock<Arc<Http>>,
    shards: OnceLock<Arc<ShardManager>>,
}

impl DiscordClient {
    pub async fn new(config: DiscordConfig) -> Self {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::GUILD_MESSAGE_REACTIONS
            | GatewayIntents::GUILD_MESSAGE_TYPING
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::DIRECT_MESSAGE_REACTIONS
            | GatewayIntents::DIRECT_MESSAGE_TYPING;

        let ret = Self {
            imp: Arc::new(DiscordClientImp {
                config,
                client: OnceLock::new(),
                http: OnceLock::new(),
                shards: OnceLock::new(),
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
        self.client_mut().await.start_autosharded().await
    }

    pub async fn stop(&self) {
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

    pub async fn onboard_user(
        &self,
        user_id: UserId,
        access_token: String,
    ) -> Result<(), serenity::Error> {
        let ret = self
            .config()
            .guild_id
            .add_member(self.http(), user_id, AddMember::new(access_token))
            .await?;

        let has_already_joined = ret.is_none();

        let ret = user_id.create_dm_channel(self.http()).await?;

        let embed = CreateEmbed::new().title("You're linked up!")
            .description(format!("You'll now get DMs from me whenever your queue is over 100.\n\n{}\n*Make sure you stay in the server to get notifications and enable DMs from server members.*", 
            if has_already_joined {
                "You've already joined the server, so you're all set!"
            }
            else {
                "You've been added to the server, so you're all set!"
            }))
            .timestamp(OffsetDateTime::now_utc())
            .color(Color::from_rgb(16, 240, 12));

        ret.send_message(self.http(), CreateMessage::new().embed(embed))
            .await?;

        Ok(())
    }
}

#[async_trait]
impl EventHandler for DiscordClient {
    async fn ready(&self, ctx: Context, _data_about_bot: serenity::model::gateway::Ready) {
        if let Err(e) = ChannelId::new(1230270710317056000)
            .say(&ctx.http, "Hello, world!")
            .await
        {
            println!("Error sending message: {:?}", e);
        }
    }
}
