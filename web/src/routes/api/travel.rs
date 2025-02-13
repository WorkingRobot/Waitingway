use crate::{models::WorldQueryFilter, storage::db};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, web, HttpResponse, Result,
};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;

pub fn service() -> impl HttpServiceFactory {
    get_travel_state
}

#[derive(Debug, Serialize)]
pub struct TravelStates {
    pub travel_time: i32,
    pub prohibited: HashMap<u16, bool>,
}

#[get("/travel/")]
async fn get_travel_state(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = get_travel_state_filtered(&pool, filter);
    let time = db::travel::get_travel_time(&pool);
    match tokio::join!(resp, time) {
        (Ok(states), Ok(time)) => Ok(HttpResponse::Ok().json(TravelStates {
            travel_time: time,
            prohibited: states,
        })),
        (Err(e), _) | (_, Err(e)) => Err(ErrorInternalServerError(e)),
    }
}

async fn get_travel_state_filtered(
    pool: &PgPool,
    filter: WorldQueryFilter,
) -> Result<HashMap<u16, bool>, sqlx::Error> {
    if let Some(region_id) = filter.region_id {
        db::travel::get_travel_states_by_region_id(pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::travel::get_travel_states_by_datacenter_id(pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::travel::get_travel_states_by_world_id(pool, world_id).await
    } else {
        db::travel::get_travel_states(pool).await
    }
}
