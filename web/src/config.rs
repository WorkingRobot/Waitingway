use serde::Deserialize;
use serenity::all::{ChannelId, GuildId};

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub client_id: u64,
    pub client_secret: String,
    pub redirect_uri: String,
    pub bot_token: String,
    pub guild_id: GuildId,
    pub log_channel_id: ChannelId,
    pub queue_size_dm_threshold: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub max_connections_per_user: u32,
    pub discord: DiscordConfig,
    #[serde(with = "hex::serde")]
    pub updates_key: [u8; 32],
}
