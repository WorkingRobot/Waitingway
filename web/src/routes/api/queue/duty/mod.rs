use crate::{
    middleware::{auth::BasicAuthentication, version::UserAgentVersion},
    models::{duty::Recap, duty::RouletteSize, RouletteQueryFilter},
    storage::db,
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, route, web, HttpResponse, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

mod notifications;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/duty")
        .service(create_roulette_size)
        .service(create_recap)
        .service(get_roulette_estimate)
        .service(get_roulette_estimate_datacenter)
        .service(notifications::service())
}

#[route("/roulette/size/", method = "POST", wrap = "BasicAuthentication")]
async fn create_roulette_size(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    size_info: web::Json<RouletteSize>,
) -> Result<HttpResponse> {
    let mut size_info = size_info.into_inner();
    size_info.user_id = *username;

    let resp = db::duty::create_roulette_size(&pool, size_info).await;
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

    let resp = db::duty::create_recap(&pool, recap).await;

    match resp {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[get("/roulette/")]
async fn get_roulette_estimate(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let resp = db::duty::get_roulette_estimates(&pool).await;
    match resp {
        Ok(estimate) => Ok(HttpResponse::Ok().json(estimate)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[get("/roulette/{datacenter_id}/")]
async fn get_roulette_estimate_datacenter(
    pool: web::Data<PgPool>,
    datacenter_id: web::Path<u16>,
    filter: actix_web_lab::extract::Query<RouletteQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = if let Some(roulette_id) = filter.roulette_id {
        db::duty::get_roulette_estimates_by_datacenter_id_filtered(
            &pool,
            *datacenter_id,
            roulette_id,
        )
        .await
    } else {
        db::duty::get_roulette_estimates_by_datacenter_id(&pool, *datacenter_id).await
    };

    match resp {
        Ok(estimate) => Ok(HttpResponse::Ok().json(estimate)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
