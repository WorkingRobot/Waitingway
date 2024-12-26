use crate::{
    db,
    db_wrappers::DatabaseDateTime,
    middleware::{auth::BasicAuthentication, version::UserAgentVersion},
    models::{
        DatacenterSummary, DbWorldInfo, DbWorldStatus, QueueEstimate, QueueSize, Recap,
        RegionSummary, Summary, WorldQueryFilter, WorldSummary,
    },
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, route, web, HttpResponse, Result,
};
use anyhow::anyhow;
use konst::{
    option,
    primitive::{parse_i64, parse_u32},
    result,
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

#[derive(Debug, Serialize)]
pub struct TravelStates {
    pub travel_time: i32,
    pub prohibited: HashMap<u16, bool>,
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
        (Err(e), _) => Err(ErrorInternalServerError(e)),
        (_, Err(e)) => Err(ErrorInternalServerError(e)),
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
async fn get_summary(pool: web::Data<PgPool>) -> Result<HttpResponse> {
    let world_info = db::get_world_info(&pool);
    let travel_states = db::get_travel_states(&pool);
    let travel_time = db::get_travel_time(&pool);
    let queue_estimates = db::get_queue_estimates(&pool);
    let world_statuses = db::get_world_statuses(&pool);
    match tokio::join!(
        world_info,
        travel_states,
        travel_time,
        queue_estimates,
        world_statuses
    ) {
        (
            Ok(world_info),
            Ok(travel_states),
            Ok(travel_time),
            Ok(queue_estimates),
            Ok(world_statuses),
        ) => Ok(HttpResponse::Ok().json(
            construct_summary(
                world_info,
                travel_states,
                travel_time,
                queue_estimates,
                world_statuses,
            )
            .map_err(ErrorInternalServerError)?,
        )),
        (Err(e), _, _, _, _)
        | (_, Err(e), _, _, _)
        | (_, _, Err(e), _, _)
        | (_, _, _, Err(e), _)
        | (_, _, _, _, Err(e)) => Err(ErrorInternalServerError(e)),
    }
}

fn construct_summary(
    world_info: Vec<DbWorldInfo>,
    travel_states: HashMap<u16, bool>,
    travel_time: i32,
    queues: Vec<QueueEstimate>,
    statuses: Vec<DbWorldStatus>,
) -> anyhow::Result<Summary> {
    let mut regions = HashMap::new();
    let mut datacenters = HashMap::new();
    let mut worlds = HashMap::new();
    for world in &world_info {
        regions
            .entry(world.region_id.0)
            .or_insert_with(|| RegionSummary {
                id: world.region_id.0,
                name: world.region_name.clone(),
                abbreviation: world.region_abbreviation.clone(),
            });

        datacenters
            .entry(world.datacenter_id.0)
            .or_insert_with(|| DatacenterSummary {
                id: world.datacenter_id.0,
                name: world.datacenter_name.clone(),
                region_id: world.region_id.0,
            });

        let world_id = world.world_id.0;
        let travel_info = travel_states
            .get(&world_id)
            .ok_or_else(|| anyhow!("No travel info {world_id}"))?;
        let queue_info = queues
            .iter()
            .find(|q| q.world_id == world_id)
            .cloned()
            .unwrap_or_else(|| QueueEstimate {
                world_id,
                last_update: DatabaseDateTime(time::OffsetDateTime::now_utc()),
                last_size: 0,
                last_duration: 0.0,
            });
        //.ok_or_else(|| anyhow!("No queue info {world_id}"))?;
        let status_info = statuses
            .iter()
            .find(|s| s.world_id.0 == world_id)
            .ok_or_else(|| anyhow!("No status info {world_id}"))?;

        worlds
            .entry(world.world_id.0)
            .or_insert_with(|| WorldSummary {
                id: world.world_id.0,
                name: world.world_name.clone(),
                datacenter_id: world.datacenter_id.0,

                travel_prohibited: *travel_info,
                world_status: status_info.status,
                world_category: status_info.category,
                world_character_creation_enabled: status_info.can_create,

                queue_size: queue_info.last_size,
                queue_duration: queue_info.last_duration,
                queue_last_update: queue_info.last_update,
            });
    }
    Ok(Summary {
        average_travel_time: travel_time,
        worlds: worlds.into_values().collect(),
        datacenters: datacenters.into_values().collect(),
        regions: regions.into_values().collect(),
    })
}
