use actix_web::{HttpResponse, HttpResponseBuilder};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheKey {
    WorldSummary,
}

pub type Cache = mini_moka::sync::Cache<CacheKey, String>;

pub trait HttpResponseBuilderExt {
    fn json_cached(&mut self, value: String) -> HttpResponse;
}

impl HttpResponseBuilderExt for HttpResponseBuilder {
    fn json_cached(&mut self, value: String) -> HttpResponse {
        self.content_type("application/json").body(value)
    }
}

pub async fn cached_response<F, Fut, T: Serialize>(
    cache: &Cache,
    key: CacheKey,
    value: F,
) -> actix_web::Result<HttpResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = actix_web::Result<T>>,
{
    if let Some(resp) = cache.get(&key) {
        return Ok(HttpResponse::Ok().json_cached(resp));
    }
    log::info!("Cache miss for {:?}", key);
    let resp = value().await;
    resp.map(|s| {
        cache.insert(key, serde_json::to_string(&s).unwrap());
        HttpResponse::Ok().json(s)
    })
}
