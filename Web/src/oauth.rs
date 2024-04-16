use std::str::FromStr;

use awc::http::{header, uri, Uri};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::config::DiscordConfig;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("Invalid base redirect URI")]
    InvalidRedirectUri(#[from] uri::InvalidUri),
    #[error("URL encode error")]
    UrlEncodeError(#[from] serde_urlencoded::ser::Error),
    #[error("URL parse error")]
    UrlParseError(#[from] uri::InvalidUriParts),
    #[error("Request error")]
    RequestError(#[from] awc::error::SendRequestError),
    #[error("JSON error")]
    JsonError(#[from] awc::error::JsonPayloadError),
}

pub fn get_redirect_uri(config: &DiscordConfig, username: Uuid) -> Result<Uri, OAuthError> {
    url_add_query(
        Uri::from_str(&config.redirect_uri)?,
        &[
            ("response_type", "code"),
            ("client_id", &config.client_id.to_string()),
            ("scope", "identify guilds.join"),
            ("state", &URL_SAFE.encode(username.as_bytes())),
            ("redirect_uri", &config.redirect_uri),
            ("prompt", "consent"),
        ],
    )
}

// Taken from https://github.com/actix/actix-web/blob/ba7fd048b601e32039ba13d95271f110980ff434/awc/src/request.rs#L292
fn url_add_query<T: Serialize>(uri: Uri, query: &T) -> Result<Uri, OAuthError> {
    let mut parts = uri.into_parts();

    if let Some(path_and_query) = parts.path_and_query {
        let query = serde_urlencoded::to_string(query)?;
        let path = path_and_query.path();
        parts.path_and_query = Some(format!("{}?{}", path, query).parse()?);
    }

    Ok(Uri::from_parts(parts)?)
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
}

pub async fn exchange_code_for_token(
    client: &awc::Client,
    config: &DiscordConfig,
    code: &str,
) -> Result<TokenResponse, OAuthError> {
    let mut response = client
        .post("https://discord.com/api/v10/oauth2/token")
        .basic_auth(config.client_id, &config.client_secret)
        .send_form(&[
            ("client_id", config.client_id.to_string()),
            ("client_secret", config.client_secret.clone()),
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", config.redirect_uri.clone()),
        ])
        .await?;

    Ok(response.json::<TokenResponse>().await?)
}

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: u64,
    pub username: String,
    pub discriminator: String,
    pub global_name: Option<String>,
}

pub async fn get_discord_identity(
    client: &awc::Client,
    token: &str,
) -> Result<DiscordUser, OAuthError> {
    let mut response = client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(token)
        .send()
        .await?;

    Ok(response.json::<DiscordUser>().await?)
}

pub async fn join_guild(
    client: &awc::Client,
    token: &str,
    guild_id: u64,
    user_id: u64,
) -> Result<(), OAuthError> {
    todo!("Twilight needs to call this. This can't be called from the web server.")
}

pub async fn kill_token_result(client: &awc::Client, token: &str) -> Result<(), OAuthError> {
    if let Some(e) = kill_token(client, token).await {
        Err(e)
    } else {
        Ok(())
    }
}

pub async fn kill_token(client: &awc::Client, token: &str) -> Option<OAuthError> {
    client
        .post("https://discord.com/api/v10/oauth2/token/revoke")
        .send_form(&[("token", token), ("token_type_hint", "access_token")])
        .await
        .err()
        .map(|e| e.into())
}
