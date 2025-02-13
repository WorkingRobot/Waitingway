use actix_web::{error::ErrorInternalServerError, HttpResponse};
use redis::{AsyncCommands, RedisResult, SetOptions};
use serde::Serialize;

use crate::storage::redis::{client::RedisClient, utils::RedisKey};

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheKey {
    WorldSummary,
}

impl RedisKey for CacheKey {
    const PREFIX: &'static str = "cache";
}

fn json_response(s: String) -> HttpResponse {
    HttpResponse::Ok().content_type("application/json").body(s)
}

async fn cached_response_imp<F, Fut, T: Serialize>(
    mut cache: RedisClient,
    key: CacheKey,
    value: F,
) -> Result<actix_web::Result<HttpResponse>, anyhow::Error>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = actix_web::Result<T>>,
{
    let rkey = key.to_key(&cache)?;
    if let Some(resp) = cache.get(&rkey).await? {
        return Ok(Ok(json_response(resp)));
    }
    log::info!("Cache miss for {:?}", key);
    match value().await {
        Ok(s) => {
            let s = serde_json::to_string(&s)?;
            let r: RedisResult<()> = cache
                .clone()
                .set_options(
                    rkey,
                    &s,
                    SetOptions::default()
                        .with_expiration(redis::SetExpiry::PX(cache.config().cache_ttl_ms)),
                )
                .await;
            r?;
            Ok(Ok(json_response(s)))
        }
        Err(e) => Ok(Err(e)),
    }
}

pub async fn cached_response<F, Fut, T: Serialize>(
    cache: RedisClient,
    key: CacheKey,
    value: F,
) -> actix_web::Result<HttpResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = actix_web::Result<T>>,
{
    match cached_response_imp(cache, key, value).await {
        Ok(r) => r,
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
