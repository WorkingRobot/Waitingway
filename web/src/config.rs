use serde::Deserialize;
use serenity::all::{ActivityData, ActivityType, ChannelId, GuildId, RoleId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub enum DiscordActivityType {
    Playing,
    // Streaming,
    Listening,
    Watching,
    Competing,
    Custom,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordActivity {
    pub r#type: DiscordActivityType,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub client_id: u64,
    pub client_secret: String,
    pub redirect_uri: String,
    pub bot_token: String,
    pub guild_id: GuildId,
    pub log_channel_id: ChannelId,
    pub connected_role_id: RoleId,
    pub queue_size_dm_threshold: u32,
    pub activities: Vec<DiscordActivity>,
    pub activity_update_interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StasisConfig {
    pub username: String,
    pub password: String,
    pub lobby_hosts: Vec<String>,
    pub uid_cache: StasisCache,
    pub dc_token_cache: StasisCache,
    pub version_file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StasisCache {
    pub path: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub metrics_server_addr: String,
    pub database_url: String,
    pub max_connections_per_user: u32,
    pub cache_ttl_ms: u64,
    pub discord: DiscordConfig,
    pub stasis: StasisConfig,
    #[serde(with = "hex::serde")]
    pub updates_key: [u8; 32],
    pub log_filter: Option<String>,
    pub log_access_format: Option<String>,
}

impl From<DiscordActivityType> for ActivityType {
    fn from(activity_type: DiscordActivityType) -> Self {
        match activity_type {
            DiscordActivityType::Playing => ActivityType::Playing,
            // DiscordActivityType::Streaming => ActivityType::Streaming,
            DiscordActivityType::Listening => ActivityType::Listening,
            DiscordActivityType::Watching => ActivityType::Watching,
            DiscordActivityType::Custom => ActivityType::Custom,
            DiscordActivityType::Competing => ActivityType::Competing,
        }
    }
}

impl From<DiscordActivity> for ActivityData {
    fn from(activity: DiscordActivity) -> Self {
        ActivityData {
            name: activity.text.clone(),
            kind: activity.r#type.into(),
            state: if activity.r#type == DiscordActivityType::Custom {
                Some(activity.text)
            } else {
                None
            },
            url: None,
        }
    }
}
