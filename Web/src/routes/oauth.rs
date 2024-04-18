use crate::{auth::BasicAuthentication, config::Config, db, discord::DiscordClient, models, oauth};
use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorBadRequest, ErrorInternalServerError},
    get,
    http::header,
    route, web, HttpResponse, Result,
};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/oauth").service(redirect).service(callback)
}

#[route("/redirect", method = "GET", wrap = "BasicAuthentication")]
async fn redirect(config: web::Data<Config>, username: web::ReqData<Uuid>) -> Result<HttpResponse> {
    Ok(HttpResponse::TemporaryRedirect()
        .insert_header((
            header::LOCATION,
            oauth::get_redirect_url(&config.discord, *username)
                .map_err(|e| ErrorInternalServerError(e))?
                .to_string(),
        ))
        .finish())
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

struct KillTokenGuard {
    client: Client,
    token: String,
}

impl KillTokenGuard {
    fn new(client: &Client, token: &str) -> Self {
        Self {
            client: client.clone(),
            token: token.to_string(),
        }
    }
}

impl Drop for KillTokenGuard {
    fn drop(&mut self) {
        let client = self.client.clone();
        let token = self.token.clone();
        tokio::task::spawn(async move { oauth::kill_token(&client, &token).await });
    }
}

#[get("/callback")]
async fn callback(
    client: web::Data<Client>,
    config: web::Data<Config>,
    discord: web::Data<DiscordClient>,
    pool: web::Data<PgPool>,
    query: web::Query<CallbackQuery>,
) -> Result<HttpResponse> {
    let username = Uuid::from_slice(
        &URL_SAFE
            .decode(&query.state)
            .map_err(|_| ErrorBadRequest("Invalid state (base64)"))?,
    )
    .map_err(|_| ErrorBadRequest("Invalid state (uuid)"))?;
    let token = oauth::exchange_code_for_token(&client, &config.discord, &query.code)
        .await
        .map_err(|e| match e {
            _ => ErrorInternalServerError(e),
        })?;
    if !token.token_type.eq_ignore_ascii_case("Bearer") {
        // Can't kill the token if it's not a bearer token
        return Err(ErrorInternalServerError("Invalid token type"));
    }
    let _guard = KillTokenGuard::new(&client, &token.access_token);

    if token.scope != "identify guilds.join" {
        return Err(ErrorBadRequest("Invalid scope"));
    }
    let identity = oauth::get_discord_identity(&client, &token.access_token)
        .await
        .map_err(|e| match e {
            _ => ErrorInternalServerError(e),
        })?;

    let conn_result = db::create_connection(
        &pool,
        models::Connection {
            user_id: username,
            created_at: PrimitiveDateTime::MIN,
            conn_user_id: models::DiscordId(identity.id.get()),
            username: identity.username.clone(),
            display_name: identity
                .global_name
                .unwrap_or_else(|| format!("{}#{}", &identity.username, &identity.discriminator)),
        },
        config.max_connections_per_user.into(),
    )
    .await;
    let mut is_unique = false;
    if let Err(e) = &conn_result {
        if let Some(e) = e.as_database_error() {
            if e.is_unique_violation() {
                is_unique = true;
            }
        }
    }
    if is_unique {
        return Err(ErrorBadRequest("You've already made this connection"));
    }
    let conn_result = conn_result.map_err(|e| ErrorInternalServerError(e))?;

    if conn_result.rows_affected() == 0 {
        return Err(ErrorBadRequest("You have too many connections already"));
    }

    discord
        .onboard_user(identity.id, token.access_token)
        .await
        .map_err(|e| ErrorInternalServerError(e))?;

    Ok(HttpResponse::NoContent().finish())
}
