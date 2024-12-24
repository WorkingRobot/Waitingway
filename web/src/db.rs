use crate::{
    db_wrappers::{DatabaseU16, DatabaseU64},
    models::{Connection, DbQueueEstimate, DbTravelState, QueueEstimate, QueueSize, Recap},
};
use sqlx::{postgres::PgQueryResult, Error, PgPool, QueryBuilder};
use std::{collections::HashMap, io};
use uuid::Uuid;

pub async fn create_recap(pool: &PgPool, recap: Recap) -> Result<(), Error> {
    // Limit the number of positions to half a week
    if recap.positions.len() > 60 * 24 * 7 {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Too many positions",
        ))));
    }

    let mut tx = pool.begin().await?;

    if recap.reentered {
        sqlx::query!(
            r#"DELETE FROM recaps WHERE user_id = $1 AND start_time = $2 AND successful IS NOT TRUE"#,
            recap.user_id,
            recap.start_time.as_db()
        )
        .execute(&mut *tx)
        .await?;
    }

    let queue_size = recap.positions.last().map(|p| p.position).unwrap_or(0);

    if queue_size == 0 {
        let queue_size_time = recap
            .positions
            .last()
            .map(|p| p.time)
            .or(recap.end_identify_time)
            .unwrap_or(recap.start_time);

        sqlx::query!(
            r#"INSERT INTO queue_sizes
            (user_id, world_id, time, size)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (world_id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                time = EXCLUDED.time,
                size = EXCLUDED.size
            WHERE queue_sizes.time < EXCLUDED.time"#r,
            recap.user_id,
            recap.world_id.as_db(),
            queue_size_time.as_db(),
            queue_size
        )
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query!(
        r#"INSERT INTO recaps
        (id, user_id, world_id, free_trial, successful, reentered, error_type, error_code, error_info, error_row, start_time, end_time, end_identify_time, client_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#r,
        recap.id,
        recap.user_id,
        recap.world_id.as_db(),
        recap.free_trial,
        recap.successful,
        recap.reentered,
        recap.error.as_ref().map(|r| r.r#type),
        recap.error.as_ref().map(|r| r.code),
        recap.error.as_ref().map(|r| &r.info),
        recap.error.as_ref().map(|r| r.error_row.as_db()),
        recap.start_time.as_db(),
        recap.end_time.as_db(),
        recap.end_identify_time.map(|t| t.as_db()),
        recap.client_version.to_string()
    )
    .execute(&mut *tx)
    .await?;

    if !recap.positions.is_empty() {
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO recap_positions (recap_id, time, identify_time, position) ",
        );
        query_builder.push_values(recap.positions, |mut b, position| {
            b.push_bind(recap.id)
                .push_bind(position.time)
                .push_bind(position.identify_time.map(|t| t.as_db()))
                .push_bind(position.position);
        });
        query_builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await
}

pub async fn create_queue_size(
    pool: &PgPool,
    size_info: QueueSize,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"INSERT INTO queue_sizes
        (user_id, world_id, time, size)
        VALUES ($1, $2, NOW() AT TIME ZONE 'UTC', $3)
        ON CONFLICT (world_id) DO UPDATE SET
            user_id = EXCLUDED.user_id,
            time = EXCLUDED.time,
            size = EXCLUDED.size"#r,
        size_info.user_id,
        size_info.world_id.as_db(),
        size_info.size
    )
    .execute(pool)
    .await
}

pub async fn create_connection(
    pool: &PgPool,
    connection: Connection,
    max_connections_per_user: i64,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"INSERT INTO connections
        (user_id, conn_user_id, username, display_name)
        SELECT $1, $2, $3, $4 WHERE (SELECT COUNT(*) FROM connections WHERE user_id = $1) < $5
        ON CONFLICT (user_id, conn_user_id) DO UPDATE SET username = EXCLUDED.username, display_name = EXCLUDED.display_name"#r,
        connection.user_id,
        connection.conn_user_id.as_db(),
        connection.username,
        connection.display_name,
        max_connections_per_user
    )
    .execute(pool)
    .await
}

pub async fn delete_connection(
    pool: &PgPool,
    user_id: Uuid,
    conn_user_id: u64,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"DELETE FROM connections
        WHERE user_id = $1 AND conn_user_id = $2"#,
        user_id,
        DatabaseU64(conn_user_id).as_db()
    )
    .execute(pool)
    .await
}

pub async fn get_connections_by_user_id(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Connection>, Error> {
    sqlx::query_as!(
        Connection,
        r#"SELECT * FROM connections WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
}

pub async fn get_connection_ids_by_user_id(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<u64>, Error> {
    Ok(sqlx::query_scalar!(
        r#"SELECT conn_user_id FROM connections WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|id| DatabaseU64::from(id).0)
    .collect())
}

pub async fn does_connection_id_exist(pool: &PgPool, connection_id: u64) -> Result<bool, Error> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM connections WHERE conn_user_id = $1)"#,
        DatabaseU64(connection_id).as_db()
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false))
}

pub async fn refresh_queue_estimates(pool: &PgPool) -> Result<PgQueryResult, Error> {
    sqlx::query!(r#"REFRESH MATERIALIZED VIEW CONCURRENTLY queue_estimates"#)
        .execute(pool)
        .await
}

pub async fn get_queue_estimates(pool: &PgPool) -> Result<Vec<QueueEstimate>, Error> {
    sqlx::query_as!(DbQueueEstimate, r#"SELECT * FROM queue_estimates"#)
        .fetch_all(pool)
        .await
        .map(|estimates| estimates.into_iter().map(QueueEstimate::from).collect())
}

pub async fn get_queue_estimates_by_region_id(
    pool: &PgPool,
    region_ids: Vec<u16>,
) -> Result<Vec<QueueEstimate>, Error> {
    let region_ids = region_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbQueueEstimate,
        r#"SELECT
            q.*
        FROM queue_estimates q
        JOIN worlds w ON q.world_id = w.world_id
        WHERE w.region_id = ANY($1)"#,
        region_ids.as_slice()
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(QueueEstimate::from).collect())
}

pub async fn get_queue_estimates_by_datacenter_id(
    pool: &PgPool,
    datacenter_ids: Vec<u16>,
) -> Result<Vec<QueueEstimate>, Error> {
    let datacenter_ids = datacenter_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbQueueEstimate,
        r#"SELECT
            q.*
        FROM queue_estimates q
        JOIN worlds w ON q.world_id = w.world_id
        WHERE w.datacenter_id = ANY($1)"#,
        datacenter_ids.as_slice()
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(QueueEstimate::from).collect())
}

pub async fn get_queue_estimates_by_world_id(
    pool: &PgPool,
    world_ids: Vec<u16>,
) -> Result<Vec<QueueEstimate>, Error> {
    let world_ids = world_ids
        .into_iter()
        .map(|id| DatabaseU16(id).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbQueueEstimate,
        r#"SELECT *
        FROM queue_estimates
        WHERE world_id = ANY($1)"#,
        world_ids.as_slice()
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(QueueEstimate::from).collect())
}

pub async fn add_travel_states(
    pool: &PgPool,
    worlds: Vec<crate::models::DCTravelWorldInfo>,
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
    Ok(
        sqlx::query_scalar!(r#"SELECT travel_time FROM travel_times ORDER BY time DESC LIMIT 1"#)
            .fetch_one(pool)
            .await?,
    )
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
