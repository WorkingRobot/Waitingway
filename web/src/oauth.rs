use crate::config::DiscordConfig;
use base64::{engine::general_purpose::URL_SAFE, Engine};
use reqwest::{Client, Url};
use serde::Deserialize;
use serenity::all::UserId;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("URL parse error")]
    UrlParseError(#[from] url::ParseError),
    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error),
}

pub fn get_redirect_url(config: &DiscordConfig, username: Uuid) -> Result<Url, OAuthError> {
    let mut url = Url::from_str("https://discord.com/oauth2/authorize")?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &config.client_id.to_string())
        .append_pair("scope", "identify applications.commands")
        .append_pair("integration_type", "1")
        .append_pair("state", &URL_SAFE.encode(username.as_bytes()))
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("prompt", "consent");
    Ok(url)
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

pub async fn exchange_code_for_token(
    client: &Client,
    config: &DiscordConfig,
    code: &str,
) -> Result<TokenResponse, OAuthError> {
    let response = client
        .post("https://discord.com/api/v10/oauth2/token")
        .basic_auth(config.client_id, Some(&config.client_secret))
        .form(&[
            ("client_id", config.client_id.to_string()),
            ("client_secret", config.client_secret.clone()),
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", config.redirect_uri.clone()),
        ])
        .send()
        .await?;

    Ok(response.json::<TokenResponse>().await?)
}

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: UserId,
    pub username: String,
    pub discriminator: String,
    pub global_name: Option<String>,
}

pub async fn get_discord_identity(client: &Client, token: &str) -> Result<DiscordUser, OAuthError> {
    let response = client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(token)
        .send()
        .await?;

    Ok(response.json::<DiscordUser>().await?)
}

pub async fn kill_token(client: &Client, token: &str) -> Result<(), OAuthError> {
    client
        .post("https://discord.com/api/v10/oauth2/token/revoke")
        .form(&[("token", token), ("token_type_hint", "access_token")])
        .send()
        .await
        .map(|_| ())?;

    Ok(())
}
