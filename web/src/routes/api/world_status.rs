use crate::{models::WorldQueryFilter, storage::db};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, web, HttpResponse, Result,
};
use sqlx::PgPool;

pub fn service() -> impl HttpServiceFactory {
    get_world_statuses
}

#[get("/world_status/")]
async fn get_world_statuses(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = if let Some(region_id) = filter.region_id {
        db::world_status::get_world_statuses_by_region_id(&pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::world_status::get_world_statuses_by_datacenter_id(&pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::world_status::get_world_statuses_by_world_id(&pool, world_id).await
    } else {
        db::world_status::get_world_statuses(&pool).await
    };

    match resp {
        Ok(statuses) => Ok(HttpResponse::Ok().json(statuses)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}
