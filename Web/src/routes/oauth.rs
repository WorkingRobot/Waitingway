use std::default;

use crate::{auth::BasicAuthentication, config::Config, db, models, oauth};
use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound},
    get,
    http::header,
    route, web, HttpResponse, Result,
};
use awc::Client;
use base64::{engine::general_purpose::URL_SAFE, Engine};
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
            oauth::get_redirect_uri(&config.discord, *username)
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

#[get("/callback")]
async fn callback(
    client: web::Data<Client>,
    config: web::Data<Config>,
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
            oauth::OAuthError::RequestError(e) => ErrorInternalServerError(e),
            oauth::OAuthError::JsonError(e) => ErrorBadRequest(e),
            _ => ErrorInternalServerError(e),
        })?;
    if !token.token_type.eq_ignore_ascii_case("Bearer") {
        // Can't kill the token if it's not a bearer token
        return Err(ErrorInternalServerError("Invalid token type"));
    }
    if token.scope != "identify guilds.join" {
        oauth::kill_token(&client, &token.access_token).await;
        return Err(ErrorBadRequest("Invalid scope"));
    }
    let identity = oauth::get_discord_identity(&client, &token.access_token)
        .await
        .map_err(|e| match e {
            oauth::OAuthError::RequestError(e) => ErrorInternalServerError(e),
            oauth::OAuthError::JsonError(e) => ErrorBadRequest(e),
            _ => ErrorInternalServerError(e),
        });
    if let Err(e) = identity {
        oauth::kill_token(&client, &token.access_token).await;
        return Err(e);
    }
    let identity = identity.unwrap();

    let conn_result = db::create_connection(
        &pool,
        models::Connection {
            user_id: username,
            created_at: PrimitiveDateTime::MIN,
            conn_user_id: models::DiscordId(identity.id),
            username: identity.username.clone(),
            display_name: identity
                .global_name
                .unwrap_or_else(|| format!("{}#{}", &identity.username, &identity.discriminator)),
        },
    )
    .await
    .map_err(|e| ErrorInternalServerError(e));
    if let Err(e) = conn_result {
        oauth::kill_token(&client, &token.access_token).await;
        return Err(e);
    }
    oauth::kill_token(&client, &token.access_token).await;

    Ok(HttpResponse::NoContent().finish())
}
