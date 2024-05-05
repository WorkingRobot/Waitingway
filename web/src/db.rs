use std::io;

use sqlx::{postgres::PgQueryResult, Error, PgPool, QueryBuilder};
use uuid::Uuid;

use crate::models::{Connection, DatabaseU64, Recap};

pub async fn create_recap(pool: &PgPool, recap: Recap) -> Result<(), Error> {
    // Limit the number of positions to 1 week
    if recap.positions.len() > 60 * 24 * 7 {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Too many positions",
        ))));
    }

    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"INSERT INTO recaps
        (id, user_id, world_id, successful, start_time, end_time)
        VALUES ($1, $2, $3, $4, $5, $6)"#r,
        recap.id,
        recap.user_id,
        recap.world_id.as_db(),
        recap.successful,
        recap.start_time,
        recap.end_time
    )
    .execute(&mut *tx)
    .await?;

    if !recap.positions.is_empty() {
        let mut query_builder =
            QueryBuilder::new("INSERT INTO recap_positions (recap_id, time, position) ");
        query_builder.push_values(recap.positions, |mut b, position| {
            b.push_bind(recap.id)
                .push_bind(position.time)
                .push_bind(position.position);
        });
        query_builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await
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
