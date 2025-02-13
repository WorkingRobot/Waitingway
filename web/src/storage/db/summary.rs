use crate::models::summary::{DbWorldSummaryInfo, WorldSummaryInfo};
use sqlx::{postgres::PgQueryResult, Error, PgPool};

pub async fn refresh_world_summaries(pool: &PgPool) -> Result<PgQueryResult, Error> {
    sqlx::query!(r#"REFRESH MATERIALIZED VIEW CONCURRENTLY world_summary"#)
        .execute(pool)
        .await
}

pub async fn get_world_summaries(pool: &PgPool) -> Result<Vec<WorldSummaryInfo>, Error> {
    sqlx::query_as!(DbWorldSummaryInfo, r#"SELECT * FROM world_summary"#)
        .fetch_all(pool)
        .await
        .map(|summary| summary.into_iter().map(WorldSummaryInfo::from).collect())
}
