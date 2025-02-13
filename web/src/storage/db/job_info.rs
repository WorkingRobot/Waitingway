use crate::{
    models::{
        duty_db::DbRouletteRole,
        job_info::{DbJobInfo, JobDisciple, JobInfo},
    },
    storage::db::wrappers::DatabaseU16,
};
use sqlx::{Error, PgPool, QueryBuilder};

pub async fn get_jobs(pool: &PgPool) -> Result<Vec<JobInfo>, Error> {
    sqlx::query_as!(
        DbJobInfo,
        r#"SELECT id, name, abbreviation, disciple AS "disciple: JobDisciple", role AS "role: DbRouletteRole", can_queue FROM jobs"#
    )
    .fetch_all(pool)
    .await
    .map(|jobs| jobs.into_iter().map(JobInfo::from).collect())
}

pub async fn upsert_jobs(pool: &PgPool, jobs: Vec<JobInfo>) -> Result<(), Error> {
    let mut query_builder = QueryBuilder::new(
        r#"--sql;
            INSERT INTO jobs (id, name, abbreviation, disciple, role, can_queue)
            "#,
    );
    query_builder.push_values(jobs, |mut b, job| {
        b.push_bind(DatabaseU16(job.id.into()).as_db())
            .push_bind(job.name)
            .push_bind(job.abbreviation)
            .push_bind(job.disciple)
            .push_bind(job.role.map(|r| r.as_db()))
            .push_bind(job.can_queue_for_duty);
    });
    query_builder.push(
        r#"
        ON CONFLICT (id) DO UPDATE
            SET name = EXCLUDED.name,
                abbreviation = EXCLUDED.abbreviation,
                disciple = EXCLUDED.disciple,
                role = EXCLUDED.role,
                can_queue = EXCLUDED.can_queue
            "#,
    );
    query_builder.build().execute(pool).await?;

    Ok(())
}
