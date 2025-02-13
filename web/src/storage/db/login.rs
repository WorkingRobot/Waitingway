use super::wrappers::DatabaseU16;
use crate::models::login::{DbQueueEstimate, QueueEstimate, QueueSize, Recap};
use sqlx::{postgres::PgQueryResult, Error, PgPool, QueryBuilder};
use std::io;

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

    let queue_size = recap.positions.last().map_or(0, |p| p.position);

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
