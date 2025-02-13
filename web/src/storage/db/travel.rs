use super::wrappers::DatabaseU16;
use crate::models::travel::{DCTravelWorldInfo, DbTravelState};
use sqlx::{Error, PgPool, QueryBuilder};
use std::collections::HashMap;

pub async fn add_travel_states(
    pool: &PgPool,
    worlds: Vec<DCTravelWorldInfo>,
    travel_time: i32,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"INSERT INTO travel_times
            (travel_time)
            VALUES ($1)"#r,
        travel_time
    )
    .execute(&mut *tx)
    .await?;

    let mut query_builder =
        QueryBuilder::new("INSERT INTO travel_states (world_id, travel, accept, prohibit) ");
    query_builder.push_values(worlds, |mut b, world| {
        b.push_bind(DatabaseU16(world.id).as_db())
            .push_bind(world.travel != 0)
            .push_bind(world.accept != 0)
            .push_bind(world.prohibit != 0);
    });
    query_builder.build().execute(&mut *tx).await?;

    tx.commit().await
}

pub async fn get_travel_time(pool: &PgPool) -> Result<i32, Error> {
    sqlx::query_scalar!(r#"SELECT travel_time FROM travel_times ORDER BY time DESC LIMIT 1"#)
        .fetch_one(pool)
        .await
}

pub async fn get_travel_states(pool: &PgPool) -> Result<HashMap<u16, bool>, Error> {
    let s = sqlx::query_as!(DbTravelState, r#"SELECT DISTINCT ON (world_id) world_id, prohibit FROM travel_states ORDER BY world_id, time DESC"#)
        .fetch_all(pool)
        .await?;
    Ok(s.into_iter()
        .map(|s| (s.world_id as u16, s.prohibit))
        .collect::<HashMap<_, _>>())
}

pub async fn get_travel_states_by_region_id(
    pool: &PgPool,
    region_ids: Vec<u16>,
) -> Result<HashMap<u16, bool>, Error> {
    let region_ids = region_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    Ok(sqlx::query_as!(
        DbTravelState,
        r#"SELECT DISTINCT ON (s.world_id)
            s.world_id, s.prohibit
        FROM travel_states s
        JOIN worlds w ON s.world_id = w.world_id
        WHERE w.region_id = ANY($1)
        ORDER BY s.world_id, s.time DESC"#,
        region_ids.as_slice()
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|s| (s.world_id as u16, s.prohibit))
    .collect::<HashMap<_, _>>())
}

pub async fn get_travel_states_by_datacenter_id(
    pool: &PgPool,
    datacenter_ids: Vec<u16>,
) -> Result<HashMap<u16, bool>, Error> {
    let datacenter_ids = datacenter_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    Ok(sqlx::query_as!(
        DbTravelState,
        r#"SELECT DISTINCT ON (s.world_id)
            s.world_id, s.prohibit
        FROM travel_states s
        JOIN worlds w ON s.world_id = w.world_id
        WHERE w.datacenter_id = ANY($1)
        ORDER BY s.world_id, s.time DESC"#,
        datacenter_ids.as_slice()
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|s| (s.world_id as u16, s.prohibit))
    .collect::<HashMap<_, _>>())
}

pub async fn get_travel_states_by_world_id(
    pool: &PgPool,
    world_ids: Vec<u16>,
) -> Result<HashMap<u16, bool>, Error> {
    let world_ids = world_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    Ok(sqlx::query_as!(
        DbTravelState,
        r#"SELECT DISTINCT ON (world_id)
            world_id, prohibit
        FROM travel_states
        WHERE world_id = ANY($1)
        ORDER BY world_id, time DESC"#,
        world_ids.as_slice()
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|s| (s.world_id as u16, s.prohibit))
    .collect::<HashMap<_, _>>())
}
