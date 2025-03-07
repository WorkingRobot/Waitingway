use super::wrappers::DatabaseU64;
use crate::models::Connection;
use sqlx::{postgres::PgQueryResult, Error, PgPool};
use uuid::Uuid;

pub async fn create_connection(
    pool: &PgPool,
    connection: Connection,
    max_connections_per_user: i64,
) -> Result<PgQueryResult, Error> {
    sqlx::query!(
        r#"--sql
        INSERT INTO connections
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
