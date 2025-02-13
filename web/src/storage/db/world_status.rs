use super::wrappers::DatabaseU16;
use crate::models::world_status::{DbWorldStatus, WorldStatusWorldInfo};
use sqlx::{postgres::PgQueryResult, Error, PgPool, QueryBuilder};

pub async fn add_world_statuses(
    pool: &PgPool,
    worlds: Vec<WorldStatusWorldInfo>,
) -> Result<PgQueryResult, Error> {
    let mut query_builder = QueryBuilder::new(
        r#"INSERT INTO world_statuses (world_id, status, category, can_create)
        SELECT worlds.world_id, data.status, data.category, data.can_create
        FROM ("#,
    );
    query_builder.push_values(worlds, |mut b, world| {
        b.push_bind(world.name)
            .push_bind(world.status)
            .push_bind(world.category)
            .push_bind(world.create);
    });
    query_builder.push(
        r#") AS data(name, status, category, can_create)
        JOIN worlds ON worlds.world_name = data.name"#,
    );
    query_builder.build().execute(pool).await
}

pub async fn get_world_statuses(pool: &PgPool) -> Result<Vec<DbWorldStatus>, Error> {
    sqlx::query_as!(
        DbWorldStatus,
        r#"SELECT DISTINCT ON (world_id)
            world_id, status, category, can_create
        FROM world_statuses
        ORDER BY world_id, time DESC"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_world_statuses_by_region_id(
    pool: &PgPool,
    region_ids: Vec<u16>,
) -> Result<Vec<DbWorldStatus>, Error> {
    let region_ids = region_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbWorldStatus,
        r#"SELECT DISTINCT ON (s.world_id)
            s.world_id, s.status, s.category, s.can_create
        FROM world_statuses s
        JOIN worlds w ON s.world_id = w.world_id
        WHERE w.region_id = ANY($1)
        ORDER BY s.world_id, s.time DESC"#,
        region_ids.as_slice()
    )
    .fetch_all(pool)
    .await
}

pub async fn get_world_statuses_by_datacenter_id(
    pool: &PgPool,
    datacenter_ids: Vec<u16>,
) -> Result<Vec<DbWorldStatus>, Error> {
    let datacenter_ids = datacenter_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbWorldStatus,
        r#"SELECT DISTINCT ON (s.world_id)
            s.world_id, s.status, s.category, s.can_create
        FROM world_statuses s
        JOIN worlds w ON s.world_id = w.world_id
        WHERE w.datacenter_id = ANY($1)
        ORDER BY s.world_id, s.time DESC"#,
        datacenter_ids.as_slice()
    )
    .fetch_all(pool)
    .await
}

pub async fn get_world_statuses_by_world_id(
    pool: &PgPool,
    world_ids: Vec<u16>,
) -> Result<Vec<DbWorldStatus>, Error> {
    let world_ids = world_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbWorldStatus,
        r#"SELECT DISTINCT ON (world_id)
            world_id, status, category, can_create
        FROM world_statuses
        WHERE world_id = ANY($1)
        ORDER BY world_id, time DESC"#,
        world_ids.as_slice()
    )
    .fetch_all(pool)
    .await
}
