use crate::{
    db,
    middleware::{auth::BasicAuthentication, version::UserAgentVersion},
    models::{QueueQueryFilter, QueueSize, Recap},
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, route, web, HttpResponse, Result,
};
use konst::{
    option,
    primitive::{parse_i64, parse_u32},
    result,
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    (
        health,
        version,
        create_queue_size,
        create_recap,
        get_queue_estimate,
    )
}

#[derive(Debug, Serialize)]
struct VersionData {
    pub name: &'static str,
    pub authors: &'static str,
    pub description: &'static str,
    pub repository: &'static str,
    pub profile: &'static str,
    pub version: &'static str,
    pub version_major: u32,
    pub version_minor: u32,
    pub version_patch: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub build_time: time::OffsetDateTime,
}

#[get("/")]
async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("OK"))
}

const VERSION_DATA: VersionData = VersionData {
    name: env!("CARGO_PKG_NAME"),
    authors: env!("CARGO_PKG_AUTHORS"),
    description: env!("CARGO_PKG_DESCRIPTION"),
    repository: env!("CARGO_PKG_REPOSITORY"),
    profile: env!("PROFILE"),
    version: env!("CARGO_PKG_VERSION"),
    version_major: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_MAJOR"))),
    version_minor: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_MINOR"))),
    version_patch: result::unwrap_ctx!(parse_u32(env!("CARGO_PKG_VERSION_PATCH"))),
    build_time: option::unwrap!(result::ok!(time::OffsetDateTime::from_unix_timestamp(
        result::unwrap_ctx!(parse_i64(env!("BUILD_TIMESTAMP")))
    ))),
};

#[get("/version/")]
async fn version() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(&VERSION_DATA))
}

#[route("/queue_size/", method = "POST", wrap = "BasicAuthentication")]
async fn create_queue_size(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    size_info: web::Json<QueueSize>,
) -> Result<HttpResponse> {
    let mut size_info = size_info.into_inner();
    size_info.user_id = *username;

    let resp = db::create_queue_size(&pool, size_info).await;
    match resp {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[route("/recap/", method = "POST", wrap = "BasicAuthentication")]
async fn create_recap(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    ua_version: UserAgentVersion,
    recap: web::Json<Recap>,
) -> Result<HttpResponse> {
    let mut recap = recap.into_inner();
    recap.client_version = ua_version;
    recap.user_id = *username;
    recap.id = Uuid::now_v7();

    let resp = db::create_recap(&pool, recap).await;

    match resp {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[get("/queue/")]
async fn get_queue_estimate(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<QueueQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = if let Some(region_id) = filter.region_id {
        db::get_queue_estimates_by_region_id(&pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::get_queue_estimates_by_datacenter_id(&pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::get_queue_estimates_by_world_id(&pool, world_id).await
    } else {
        db::get_queue_estimates(&pool).await
    };

    match resp {
        Ok(estimate) => Ok(HttpResponse::Ok().json(estimate)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
