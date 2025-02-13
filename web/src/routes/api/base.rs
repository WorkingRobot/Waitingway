use crate::natives::VERSION_DATA;
use actix_web::{dev::HttpServiceFactory, get, HttpResponse, Result};

pub fn service() -> impl HttpServiceFactory {
    (health, version)
}

#[get("/")]
async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("OK"))
}

#[get("/version/")]
async fn version() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(&VERSION_DATA))
}
