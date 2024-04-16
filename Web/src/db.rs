use sqlx::{postgres::PgQueryResult, Error, PgPool};
use uuid::Uuid;

use crate::models::{Connection, Recap};

pub async fn create_recap(pool: &PgPool, recap: Recap) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"INSERT INTO recaps
        (id, user_id, world_id, successful, start_time, end_time)
        VALUES ($1, $2, $3, $4, $5, $6)"#r,
        recap.id,
        recap.user_id,
        recap.world_id,
        recap.successful,
        recap.start_time,
        recap.end_time
    )
    .execute(pool)
    .await
}

pub async fn create_connection(
    pool: &PgPool,
    connection: Connection,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"INSERT INTO connections
        (user_id, conn_user_id, username, display_name)
        VALUES ($1, $2, $3, $4)"#r,
        connection.user_id,
        connection.conn_user_id.as_db(),
        connection.username,
        connection.display_name
    )
    .execute(pool)
    .await
}

pub async fn delete_connection(
    pool: &PgPool,
    user_id: Uuid,
    conn_user_id: i64,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"DELETE FROM connections
        WHERE user_id = $1 AND conn_user_id = $2"#,
        user_id,
        conn_user_id
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
) -> Result<Vec<i64>, Error> {
    sqlx::query_scalar!(
        r#"SELECT conn_user_id FROM connections WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
}
