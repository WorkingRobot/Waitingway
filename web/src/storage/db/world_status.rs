use super::wrappers::DatabaseU16;
use crate::models::world_status::{DbWorldStatus, WorldStatusWorldInfo};
use sqlx::{postgres::PgQueryResult, Error, PgPool, QueryBuilder};

pub async fn add_world_statuses(
    pool: &PgPool,
    worlds: Vec<WorldStatusWorldInfo>,
) -> Result<PgQueryResult, Error> {
    let prefix = "WITH input_data (world_name, status, category, can_create) AS (";
    let suffix = r#"
    ),
    new_data AS (
        SELECT w.world_id, i.status, i.category, i.can_create
        FROM input_data i
        JOIN worlds w ON w.world_name = i.world_name
    ),
    filtered_ids AS (
        SELECT DISTINCT n.world_id
        FROM new_data n
        CROSS JOIN LATERAL (
            SELECT status, category, can_create
            FROM world_statuses t
            WHERE t.world_id = n.world_id
            ORDER BY t.time DESC
            LIMIT 1
        ) t
        WHERE t.status IS DISTINCT FROM n.status
        OR t.category IS DISTINCT FROM n.category 
        OR t.can_create IS DISTINCT FROM n.can_create
    )
    INSERT INTO world_statuses (world_id, status, category, can_create)
    SELECT n.world_id, n.status, n.category, n.can_create
    FROM new_data n
    JOIN filtered_ids f USING (world_id);"#;

    let mut query_builder = QueryBuilder::new(prefix);
    query_builder.push_values(worlds, |mut b, world| {
        b.push_bind(world.name)
            .push_bind(world.status)
            .push_bind(world.category)
            .push_bind(world.create);
    });
    query_builder.push(suffix);
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
