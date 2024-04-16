use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub client_id: u64,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub discord: DiscordConfig,
}
