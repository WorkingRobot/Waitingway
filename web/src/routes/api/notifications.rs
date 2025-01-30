use crate::{
    config::Config, discord::DiscordClient, middleware::auth::BasicAuthentication, storage::db,
};
use actix_web::{
    dev::{HttpServiceFactory, Payload},
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized, JsonPayloadError},
    route, web, FromRequest, HttpMessage, HttpRequest, HttpResponse, HttpResponseBuilder, Result,
};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, KeyInit, XChaCha20Poly1305, XNonce,
};
use futures_util::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, MessageId, UserId};
use sqlx::PgPool;
use tokio::task::JoinSet;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/notifications")
        .service(create)
        .service(update)
        .service(delete)
}

#[derive(Clone, Debug, Deserialize)]
struct CreateData {
    pub character_name: String,
    pub home_world_id: u16,
    pub world_id: u16,
    #[serde(flatten)]
    pub update_data: UpdateData,
}

#[derive(Clone, Debug, Deserialize)]
struct UpdateData {
    pub position: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub estimated_time: time::OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize)]
struct DeleteData {
    pub successful: bool,
    pub queue_start_size: u32,
    pub queue_end_size: u32,
    #[serde(rename = "duration")]
    pub duration_secs: u32,
    pub error_message: Option<String>,
    pub error_code: Option<i32>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub identify_timeout: Option<time::OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstanceData {
    pub username: Uuid,
    pub character_name: String,
    pub home_world_id: u16,
    pub world_id: u16,
    pub messages: Vec<(MessageId, ChannelId)>,
}

impl FromRequest for InstanceData {
    type Error = actix_web::Error;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let config = req
                .app_data::<web::Data<Config>>()
                .ok_or(ErrorInternalServerError("Config missing"))?;
            let nonce = req
                .headers()
                .get("X-Instance-Nonce")
                .ok_or(ErrorBadRequest("No instance nonce provided"))?
                .to_str()
                .map_err(|_| ErrorBadRequest("Invalid instance nonce (header)"))?;
            let nonce = URL_SAFE
                .decode(nonce)
                .map_err(|_| ErrorBadRequest("Invalid instance nonce (base64)"))?;
            let nonce = XNonce::from_exact_iter(nonce.into_iter())
                .ok_or(ErrorBadRequest("Invalid instance nonce (size)"))?;
            let data = req
                .headers()
                .get("X-Instance-Data")
                .ok_or(ErrorBadRequest("No instance data provided"))?
                .to_str()
                .map_err(|_| ErrorBadRequest("Invalid instance data (header)"))?;
            let data = URL_SAFE
                .decode(data)
                .map_err(|_| ErrorBadRequest("Invalid instance data (base64)"))?;
            let key = &config.updates_key;
            let key = XChaCha20Poly1305::new(key.into());
            let plaintext: Vec<u8> = key
                .decrypt(&nonce, data.as_slice())
                .map_err(|_| ErrorBadRequest("Invalid instance data (bad encryption)"))?;
            let data: InstanceData = serde_json::from_slice(plaintext.as_slice())
                .map_err(|_| ErrorBadRequest("Invalid instance data (not json)"))?;

            let req_username = *req
                .extensions()
                .get::<Uuid>()
                .ok_or(ErrorUnauthorized("No username provided"))?;
            if data.username != req_username {
                return Err(ErrorBadRequest("Invalid instance data (username mismatch)"));
            }

            Ok(data)
        })
    }
}

impl InstanceData {
    pub fn append_data(
        &self,
        config: &Config,
        builder: &mut HttpResponseBuilder,
    ) -> Result<(), HttpResponse> {
        let key = &config.updates_key;
        let key = XChaCha20Poly1305::new(key.into());
        let data = serde_json::to_string(self)
            .map_err(|err| HttpResponse::from_error(JsonPayloadError::Serialize(err)))?;

        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = key
            .encrypt(&nonce, data.as_bytes())
            .map_err(|_| ErrorInternalServerError("Failed to encrypt instance data"))?;

        let ciphertext = URL_SAFE.encode(ciphertext);
        let nonce = URL_SAFE.encode(nonce);

        builder.append_header(("X-Instance-Nonce", nonce));
        builder.append_header(("X-Instance-Data", ciphertext));

        Ok(())
    }
}

#[route("/", method = "POST", wrap = "BasicAuthentication")]
async fn create(
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    discord: web::Data<DiscordClient>,
    username: web::ReqData<Uuid>,
    data: web::Json<CreateData>,
) -> Result<HttpResponse> {
    if data.update_data.position < config.discord.queue_size_dm_threshold {
        return Ok(HttpResponse::NoContent().finish());
    }

    let connections = db::get_connection_ids_by_user_id(&pool, *username)
        .await
        .map_err(ErrorInternalServerError)?;

    let discord = discord.into_inner();
    let data = data.into_inner();

    let mut joinset = JoinSet::new();
    for user_id in connections {
        let discord = discord.clone();
        let character_name = data.character_name.clone();
        let data = data.update_data.clone();
        joinset.spawn(async move {
            discord
                .send_queue_position(
                    UserId::new(user_id),
                    &character_name,
                    data.position,
                    data.updated_at,
                    data.estimated_time,
                )
                .await
        });
    }

    let mut messages = vec![];

    while let Some(ret) = joinset.join_next().await {
        match ret {
            Ok(Ok(m)) => messages.push((m.id, m.channel_id)),
            Ok(Err(e)) => return Err(ErrorInternalServerError(e)),
            Err(e) => return Err(ErrorInternalServerError(e)),
        }
    }

    let data = InstanceData {
        username: *username,
        character_name: data.character_name,
        home_world_id: data.home_world_id,
        world_id: data.world_id,
        messages,
    };

    let mut resp = HttpResponse::Created();

    if let Err(e) = data.append_data(&config, &mut resp) {
        return Ok(e);
    }

    Ok(resp.finish())
}

#[route("/", method = "PATCH", wrap = "BasicAuthentication")]
async fn update(
    instance_data: InstanceData,
    discord: web::Data<DiscordClient>,
    data: web::Json<UpdateData>,
) -> Result<HttpResponse> {
    let discord = discord.into_inner();
    let data = data.into_inner();

    let mut joinset = JoinSet::new();
    for (message_id, channel_id) in instance_data.messages {
        let discord = discord.clone();
        let character_name = instance_data.character_name.clone();
        let data = data.clone();
        joinset.spawn(async move {
            discord
                .update_queue_position(
                    message_id,
                    channel_id,
                    &character_name,
                    data.position,
                    data.updated_at,
                    data.estimated_time,
                )
                .await
        });
    }

    while let Some(ret) = joinset.join_next().await {
        match ret {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(ErrorInternalServerError(e)),
            Err(e) => return Err(ErrorInternalServerError(e)),
        }
    }

    Ok(HttpResponse::NoContent().finish())
}

#[route("/", method = "DELETE", wrap = "BasicAuthentication")]
async fn delete(
    instance_data: InstanceData,
    discord: web::Data<DiscordClient>,
    data: web::Json<DeleteData>,
) -> Result<HttpResponse> {
    let discord = discord.into_inner();
    let data = data.into_inner();

    let mut joinset = JoinSet::new();
    for (message_id, channel_id) in instance_data.messages {
        let discord = discord.clone();
        let character_name = instance_data.character_name.clone();
        let data = data.clone();
        joinset.spawn(async move {
            discord
                .send_queue_completion(
                    message_id,
                    channel_id,
                    &character_name,
                    data.queue_start_size,
                    data.queue_end_size,
                    time::Duration::new(data.duration_secs.into(), 0),
                    data.error_message,
                    data.error_code,
                    data.identify_timeout,
                    data.successful,
                )
                .await
        });
    }

    while let Some(ret) = joinset.join_next().await {
        match ret {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(ErrorInternalServerError(e)),
            Err(e) => return Err(ErrorInternalServerError(e)),
        }
    }

    Ok(HttpResponse::NoContent().finish())
}
