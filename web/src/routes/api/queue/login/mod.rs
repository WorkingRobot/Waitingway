use crate::{
    middleware::{auth::BasicAuthentication, version::UserAgentVersion},
    models::{login::QueueSize, login::Recap, WorldQueryFilter},
    storage::db,
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, web, HttpResponse, Resource, Result,
};
use sqlx::PgPool;
use uuid::Uuid;

mod notifications;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/login")
        .service(
            Resource::new("/size/").route(web::post().wrap(BasicAuthentication).to(create_size)),
        )
        .service(
            Resource::new("/recap/").route(web::post().wrap(BasicAuthentication).to(create_recap)),
        )
        .service(Resource::new("/").route(web::get().to(get_queue_estimate)))
        .service(notifications::service())
}

pub fn service_v1() -> impl HttpServiceFactory {
    (
        Resource::new("/queue_size/").route(web::post().wrap(BasicAuthentication).to(create_size)),
        Resource::new("/recap/").route(web::post().wrap(BasicAuthentication).to(create_recap)),
        Resource::new("/queue/").route(web::get().to(get_queue_estimate)),
        notifications::service(),
    )
}

//#[route("/size/", method = "POST", wrap = "BasicAuthentication")]
async fn create_size(
    pool: web::Data<PgPool>,
    username: web::ReqData<Uuid>,
    size_info: web::Json<QueueSize>,
) -> Result<HttpResponse> {
    let mut size_info = size_info.into_inner();
    size_info.user_id = *username;

    let resp = db::login::create_queue_size(&pool, size_info).await;
    match resp {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

//#[route("/recap/", method = "POST", wrap = "BasicAuthentication")]
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

    let resp = db::login::create_recap(&pool, recap).await;

    match resp {
        Ok(_) => Ok(HttpResponse::Created().finish()),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

//#[get("/")]
async fn get_queue_estimate(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = if let Some(region_id) = filter.region_id {
        db::login::get_queue_estimates_by_region_id(&pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::login::get_queue_estimates_by_datacenter_id(&pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::login::get_queue_estimates_by_world_id(&pool, world_id).await
    } else {
        db::login::get_queue_estimates(&pool).await
    };

    match resp {
        Ok(estimate) => Ok(HttpResponse::Ok().json(estimate)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
