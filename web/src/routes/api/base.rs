use crate::{
    cache::{cached_response, Cache, CacheKey},
    db,
    middleware::{auth::BasicAuthentication, version::UserAgentVersion},
    models::{
        DatacenterSummary, QueueSize, Recap, RegionSummary, Summary, WorldQueryFilter,
        WorldSummary, WorldSummaryInfo,
    },
    natives::VERSION_DATA,
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, route, web, HttpResponse, Result,
};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    (
        health,
        version,
        create_queue_size,
        create_recap,
        get_queue_estimate,
        get_travel_state,
        get_world_statuses,
        get_summary,
    )
}

#[derive(Debug, Serialize)]
pub struct TravelStates {
    pub travel_time: i32,
    pub prohibited: HashMap<u16, bool>,
}

#[get("/")]
async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().body("OK"))
}

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
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
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

#[get("/travel/")]
async fn get_travel_state(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = get_travel_state_filtered(&pool, filter);
    let time = db::get_travel_time(&pool);
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
        db::get_travel_states_by_region_id(pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::get_travel_states_by_datacenter_id(pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::get_travel_states_by_world_id(pool, world_id).await
    } else {
        db::get_travel_states(pool).await
    }
}

#[get("/world_status/")]
async fn get_world_statuses(
    pool: web::Data<PgPool>,
    filter: actix_web_lab::extract::Query<WorldQueryFilter>,
) -> Result<HttpResponse> {
    let filter = filter.into_inner();
    let resp = if let Some(region_id) = filter.region_id {
        db::get_world_statuses_by_region_id(&pool, region_id).await
    } else if let Some(datacenter_id) = filter.datacenter_id {
        db::get_world_statuses_by_datacenter_id(&pool, datacenter_id).await
    } else if let Some(world_id) = filter.world_id {
        db::get_world_statuses_by_world_id(&pool, world_id).await
    } else {
        db::get_world_statuses(&pool).await
    };

    match resp {
        Ok(statuses) => Ok(HttpResponse::Ok().json(statuses)),
        Err(e) => Err(ErrorInternalServerError(e)),
    }
}

#[get("/summary/")]
async fn get_summary(pool: web::Data<PgPool>, cache: web::Data<Cache>) -> Result<HttpResponse> {
    cached_response(&cache, CacheKey::WorldSummary, || async {
        let world_summaries = db::get_world_summaries(&pool);
        let travel_time = db::get_travel_time(&pool);
        match tokio::join!(world_summaries, travel_time) {
            (Ok(world_summaries), Ok(travel_time)) => {
                Ok(construct_summary(&world_summaries, travel_time))
            }
            (Err(e), _) | (_, Err(e)) => Err(ErrorInternalServerError(e)),
        }
    })
    .await
}

fn construct_summary(world_summaries: &[WorldSummaryInfo], travel_time: i32) -> Summary {
    let mut regions = HashMap::new();
    let mut datacenters = HashMap::new();
    let mut worlds = HashMap::new();
    for world in world_summaries {
        regions
            .entry(world.region_id)
            .or_insert_with(|| RegionSummary {
                id: world.region_id,
                name: world.region_name.clone(),
                abbreviation: world.region_abbreviation.clone(),
            });

        datacenters
            .entry(world.datacenter_id)
            .or_insert_with(|| DatacenterSummary {
                id: world.datacenter_id,
                name: world.datacenter_name.clone(),
                region_id: world.region_id,
            });

        worlds
            .entry(world.world_id)
            .or_insert_with(|| WorldSummary {
                id: world.world_id,
                name: world.world_name.clone(),
                datacenter_id: world.datacenter_id,

                travel_prohibited: world.travel_prohibit,
                world_status: world.status,
                world_category: world.category,
                world_character_creation_enabled: world.can_create,

                queue_size: world.queue_size,
                queue_duration: world.queue_duration,
                queue_last_update: world.queue_time,
            });
    }
    Summary {
        average_travel_time: travel_time,
        worlds: worlds.into_values().collect(),
        datacenters: datacenters.into_values().collect(),
        regions: regions.into_values().collect(),
    }
}
