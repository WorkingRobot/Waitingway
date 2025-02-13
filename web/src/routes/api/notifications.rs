use crate::{
    config::Config, discord::DiscordClient, middleware::auth::BasicAuthentication, storage::db,
};
use actix_web::{
    dev::HttpServiceFactory,
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized, JsonPayloadError},
    web, FromRequest, HttpMessage, HttpRequest, HttpResponse, HttpResponseBuilder, Result,
};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, KeyInit, XChaCha20Poly1305, XNonce,
};
use futures_util::future::LocalBoxFuture;
use serde::{de::DeserializeOwned, Serialize};
use serenity::{
    all::{ChannelId, Message, MessageId, UserId},
    async_trait,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinSet;
use uuid::Uuid;

pub use crate::impl_notification_instance;

pub type FromReqError = actix_web::Error;
pub type FromReqFuture<T> = LocalBoxFuture<'static, Result<T, FromReqError>>;

#[async_trait]
pub trait NotificationInstance: Sync + Send + Serialize + DeserializeOwned + FromRequest
where
    Self: 'static,
{
    type CreateData: Sync + Send + DeserializeOwned + 'static;
    type UpdateData: Sync + Send + DeserializeOwned + 'static;
    type DeleteData: Sync + Send + DeserializeOwned + 'static;

    fn new(username: Uuid, messages: Vec<(MessageId, ChannelId)>, data: &Self::CreateData) -> Self;

    fn username(&self) -> &Uuid;

    fn messages(&self) -> &Vec<(MessageId, ChannelId)>;

    fn passes_threshold(data: &Self::CreateData, config: &Config) -> bool;

    async fn dispatch_create(
        data: &Self::CreateData,
        discord: &DiscordClient,
        id: UserId,
    ) -> Result<Message, serenity::Error>;

    async fn dispatch_update(
        &self,
        data: &Self::UpdateData,
        discord: &DiscordClient,
        message: MessageId,
        channel: ChannelId,
    ) -> Result<(), serenity::Error>;

    async fn dispatch_delete(
        &self,
        data: &Self::DeleteData,
        discord: &DiscordClient,
        message: MessageId,
        channel: ChannelId,
    ) -> Result<(), serenity::Error>;

    fn service() -> impl HttpServiceFactory {
        web::resource("/")
            .wrap(BasicAuthentication)
            .route(web::post().to(create::<Self>))
            .route(web::patch().to(update::<Self>))
            .route(web::delete().to(delete::<Self>))
    }

    fn from_request(req: &HttpRequest) -> FromReqFuture<Self> {
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
            let data: Self = serde_json::from_slice(plaintext.as_slice())
                .map_err(|_| ErrorBadRequest("Invalid instance data (not json)"))?;

            let req_username = *req
                .extensions()
                .get::<Uuid>()
                .ok_or(ErrorUnauthorized("No username provided"))?;
            if *data.username() != req_username {
                return Err(ErrorBadRequest("Invalid instance data (username mismatch)"));
            }

            Ok(data)
        })
    }

    fn append_to_response(
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

pub async fn create<D: NotificationInstance>(
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    discord: web::Data<DiscordClient>,
    username: web::ReqData<Uuid>,
    data: web::Json<D::CreateData>,
) -> Result<HttpResponse> {
    if !D::passes_threshold(&data, &config) {
        return Ok(HttpResponse::NoContent().finish());
    }

    let connections = db::connections::get_connection_ids_by_user_id(&pool, *username)
        .await
        .map_err(ErrorInternalServerError)?;

    let discord = discord.into_inner();
    let data = data.into_inner();

    let mut joinset = JoinSet::new();
    let data = Arc::new(data);
    for user_id in connections {
        let discord = discord.clone();
        let data = data.clone();
        joinset
            .spawn(async move { D::dispatch_create(&data, &discord, UserId::new(user_id)).await });
    }

    let mut messages = vec![];

    while let Some(ret) = joinset.join_next().await {
        match ret {
            Ok(Ok(m)) => messages.push((m.id, m.channel_id)),
            Ok(Err(e)) => return Err(ErrorInternalServerError(e)),
            Err(e) => return Err(ErrorInternalServerError(e)),
        }
    }

    let data = D::new(*username, messages, &*data);

    let mut resp = HttpResponse::Created();

    if let Err(e) = data.append_to_response(&config, &mut resp) {
        return Ok(e);
    }

    Ok(resp.finish())
}

pub async fn update<D: NotificationInstance + 'static>(
    instance_data: D,
    discord: web::Data<DiscordClient>,
    data: web::Json<D::UpdateData>,
) -> Result<HttpResponse> {
    let discord = discord.into_inner();
    let data = data.into_inner();
    let data = Arc::new(data);
    let instance_data = Arc::new(instance_data);

    let mut joinset = JoinSet::new();
    for (message_id, channel_id) in instance_data.messages().clone() {
        let discord = discord.clone();
        let instance_data = instance_data.clone();
        let data = data.clone();
        joinset.spawn(async move {
            instance_data
                .dispatch_update(&data, &discord, message_id, channel_id)
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

pub async fn delete<D: NotificationInstance + 'static>(
    instance_data: D,
    discord: web::Data<DiscordClient>,
    data: web::Json<D::DeleteData>,
) -> Result<HttpResponse> {
    let discord = discord.into_inner();
    let data = data.into_inner();
    let data = Arc::new(data);
    let instance_data = Arc::new(instance_data);

    let mut joinset = JoinSet::new();
    for (message_id, channel_id) in instance_data.messages().clone() {
        let discord = discord.clone();
        let instance_data = instance_data.clone();
        let data = data.clone();
        joinset.spawn(async move {
            instance_data
                .dispatch_delete(&data, &discord, message_id, channel_id)
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

#[macro_export]
macro_rules! impl_notification_instance {
    ($ty:ty) => {
        impl FromRequest for $ty {
            type Error = $crate::routes::api::notifications::FromReqError;

            type Future = $crate::routes::api::notifications::FromReqFuture<Self>;

            fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
                <Self as NotificationInstance>::from_request(req)
            }
        }
    };
}
