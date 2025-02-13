use crate::{
    cache::{cached_response, CacheKey},
    models::summary::{DatacenterSummary, RegionSummary, Summary, WorldSummary, WorldSummaryInfo},
    storage::{db, redis::client::RedisClient},
};
use actix_web::{
    dev::HttpServiceFactory, error::ErrorInternalServerError, get, web, HttpResponse, Result,
};
use sqlx::PgPool;
use std::collections::HashMap;

pub fn service() -> impl HttpServiceFactory {
    get_summary
}

#[get("/summary/")]
async fn get_summary(
    pool: web::Data<PgPool>,
    cache: web::Data<RedisClient>,
) -> Result<HttpResponse> {
    cached_response((**cache).clone(), CacheKey::WorldSummary, || async {
        let world_summaries = db::summary::get_world_summaries(&pool);
        let travel_time = db::travel::get_travel_time(&pool);
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
