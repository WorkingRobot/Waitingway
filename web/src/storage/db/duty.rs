use super::wrappers::{DatabaseDateTime, DatabaseU16};
use crate::{
    models::{
        duty::{
            PartyMember, QueueLanguage, Recap, RecapUpdateData, RouletteEstimate, RoulettePosition,
            RouletteSize, WaitTime,
        },
        duty_db::{DbRecapUpdateType, DbRouletteEstimate, DbRouletteRole},
    },
    storage::game::{jobs, worlds},
};
use itertools::Itertools;
use sqlx::{Error, PgPool, QueryBuilder};
use std::io;

pub async fn create_recap(pool: &PgPool, recap: Recap) -> Result<(), Error> {
    // Limit the number of updates to 1000
    if recap.updates.len() > 1000 {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Too many updates",
        ))));
    }

    // Limit the number of pops to 50
    if recap.pops.len() > 50 {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Too many pops",
        ))));
    }

    if recap.queued_content.is_none() == recap.queued_roulette.is_none() {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Exactly one of queued_content or queued_roulette must be set",
        ))));
    }

    let datacenter_id = if let Some(world_id) = worlds::get_data().get_world_by_id(recap.world_id) {
        world_id.datacenter.id
    } else {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid world id",
        ))));
    };

    let Some(queued_job) = jobs::get_data().get_job_by_id(recap.queued_job) else {
        return Err(Error::Decode(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid job id",
        ))));
    };

    let mut tx = pool.begin().await?;

    // Solo roulette queues only
    if recap.party.is_none() {
        if let Some(roulette) = recap.queued_roulette {
            let Some(role) = queued_job.role else {
                return Err(Error::Decode(Box::new(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Invalid job id (No associated role)",
                ))));
            };

            let mut roulette_updates = recap
                .updates
                .iter()
                .filter(|u| matches!(u.update_data, Some(RecapUpdateData::Roulette { .. })))
                .collect_vec();
            let first_pop_time = recap.pops.first().map(|p| p.time);
            if let Some(first_pop_time) = first_pop_time {
                roulette_updates = roulette_updates
                    .into_iter()
                    .take_while(|u| u.time < first_pop_time)
                    .collect_vec();
            }

            let size = roulette_updates.iter().find_map(|u| match u.update_data {
                Some(RecapUpdateData::Roulette { position, .. })
                    if position != RoulettePosition::RetrievingInfo =>
                {
                    Some((u.time, position))
                }
                _ => None,
            });
            let estimated_wait = roulette_updates
                .iter()
                .rev()
                .find_map(|u| match u.update_data {
                    Some(RecapUpdateData::Roulette { wait_time, .. })
                        if wait_time != WaitTime::Hidden =>
                    {
                        Some((u.time, wait_time))
                    }
                    _ => None,
                });
            let wait = first_pop_time.map(|t| (t, t.0 - recap.start_time.0));

            if let Some((time, size)) = size {
                sqlx::query!(
                    r#"--sql
                    INSERT INTO roulette_sizes
                    (
                        datacenter_id, roulette_id, role,
                        size_user_id, size_time, size
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (datacenter_id, roulette_id, role) DO UPDATE SET
                        size_user_id = EXCLUDED.size_user_id,
                        size_time = EXCLUDED.size_time,
                        size = EXCLUDED.size
                    WHERE roulette_sizes.size_time < EXCLUDED.size_time"#r,
                    DatabaseU16(datacenter_id).as_db(),
                    DatabaseU16(u16::from(roulette)).as_db(),
                    role.as_db() as DbRouletteRole,
                    recap.user_id,
                    time.as_db(),
                    DatabaseU16(u16::from(u8::from(size))).as_db()
                )
                .execute(&mut *tx)
                .await?;
            }

            if let Some((time, est_wait)) = estimated_wait {
                sqlx::query!(
                    r#"--sql
                    INSERT INTO roulette_sizes
                    (
                        datacenter_id, roulette_id, role,
                        est_time_user_id, est_time_time, est_time
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (datacenter_id, roulette_id, role) DO UPDATE SET
                        est_time_user_id = EXCLUDED.est_time_user_id,
                        est_time_time = EXCLUDED.est_time_time,
                        est_time = EXCLUDED.est_time
                    WHERE roulette_sizes.est_time_time < EXCLUDED.est_time_time"#r,
                    DatabaseU16(datacenter_id).as_db(),
                    DatabaseU16(u16::from(roulette)).as_db(),
                    role.as_db() as DbRouletteRole,
                    recap.user_id,
                    time.as_db(),
                    DatabaseU16(u16::from(u8::from(est_wait))).as_db()
                )
                .execute(&mut *tx)
                .await?;
            }

            if let Some((time, wait)) = wait {
                sqlx::query!(
                    r#"--sql
                    INSERT INTO roulette_sizes
                    (
                        datacenter_id, roulette_id, role,
                        wait_time_user_id, wait_time_time, wait_time
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (datacenter_id, roulette_id, role) DO UPDATE SET
                        wait_time_user_id = EXCLUDED.wait_time_user_id,
                        wait_time_time = EXCLUDED.wait_time_time,
                        wait_time = EXCLUDED.wait_time
                    WHERE roulette_sizes.wait_time_time < EXCLUDED.wait_time_time"#r,
                    DatabaseU16(datacenter_id).as_db(),
                    DatabaseU16(u16::from(roulette)).as_db(),
                    role.as_db() as DbRouletteRole,
                    recap.user_id,
                    time.as_db(),
                    wait.as_seconds_f64()
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    let members = recap.party.as_ref().map(|p| &p.members);
    let queued_content = recap
        .queued_content
        .as_ref()
        .map(|c| c.iter().map(|v| DatabaseU16(*v).as_db()).collect_vec());

    sqlx::query!(
        r#"--sql;
        INSERT INTO duty_recaps
        (
            id, user_id,
            queued_roulette, queued_content, queued_job, queued_flags,
            world_id, is_party_leader, party_members,
            start_time, end_time, withdraw_message, client_version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#r,
        recap.id,
        recap.user_id,
        recap.queued_roulette.map(|v| DatabaseU16(v.into()).as_db()),
        queued_content.as_deref(),
        DatabaseU16(recap.queued_job.into()).as_db(),
        DatabaseU16(recap.queued_flags.as_flags(recap.queued_languages)).as_db(),
        DatabaseU16(recap.world_id).as_db(),
        recap.party.as_ref().map_or(false, |p| p.is_party_leader),
        members as Option<&Vec<PartyMember>>,
        recap.start_time.as_db(),
        recap.end_time.as_db(),
        recap.withdraw_message.map(|v| DatabaseU16(v).as_db()),
        recap.client_version.to_string()
    )
    .execute(&mut *tx)
    .await?;

    if !recap.updates.is_empty() {
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO duty_updates (recap_id, time, reserving, update_type, wait_time, position, fill_params) ",
        );
        query_builder.push_values(recap.updates, |mut b, update| {
            let update_type = match update.update_data {
                Some(RecapUpdateData::WaitTime { .. }) => DbRecapUpdateType::WaitTime,
                Some(RecapUpdateData::Players { .. }) => DbRecapUpdateType::Players,
                Some(RecapUpdateData::Thd { .. }) => DbRecapUpdateType::Thd,
                Some(RecapUpdateData::Roulette { .. }) => DbRecapUpdateType::Roulette,
                None => DbRecapUpdateType::None,
            };
            let wait_time = match update.update_data {
                Some(
                    RecapUpdateData::WaitTime { wait_time }
                    | RecapUpdateData::Players { wait_time, .. }
                    | RecapUpdateData::Thd { wait_time, .. }
                    | RecapUpdateData::Roulette { wait_time, .. },
                ) => Some(wait_time),
                None => None,
            }
            .map(|t| DatabaseU16(u8::from(t).into()).as_db());
            let position = match update.update_data {
                Some(RecapUpdateData::Roulette { position, .. }) => Some(position),
                _ => None,
            }
            .map(|t| DatabaseU16(u8::from(t).into()).as_db());
            let fill_params = match update.update_data {
                Some(RecapUpdateData::Thd {
                    tanks,
                    healers,
                    dps,
                    ..
                }) => Some(vec![tanks, healers, dps]),
                Some(RecapUpdateData::Players { players, .. }) => Some(vec![players]),
                _ => None,
            };
            b.push_bind(recap.id)
                .push_bind(update.time)
                .push_bind(update.is_reserving_server)
                .push_bind(update_type)
                .push_bind(wait_time)
                .push_bind(position)
                .push_bind(fill_params);
        });
        query_builder.build().execute(&mut *tx).await?;
    }

    if !recap.pops.is_empty() {
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO duty_pops (recap_id, time, flags, content, in_progress_time) ",
        );
        query_builder.push_values(recap.pops, |mut b, pop| {
            b.push_bind(recap.id)
                .push_bind(pop.time)
                .push_bind(DatabaseU16(pop.resulting_flags.as_flags(QueueLanguage::None)).as_db())
                .push_bind(pop.resulting_content.map(|v| DatabaseU16(v).as_db()))
                .push_bind(pop.in_progress_time);
        });
        query_builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await
}

pub async fn create_roulette_size(pool: &PgPool, size_info: RouletteSize) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    let datacenter_id =
        if let Some(world_id) = worlds::get_data().get_world_by_id(size_info.world_id) {
            world_id.datacenter.id
        } else {
            return Err(Error::Decode(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid world id",
            ))));
        };

    if let Some(size) = size_info.size {
        sqlx::query!(
            r#"--sql
                INSERT INTO roulette_sizes
                (
                    datacenter_id, roulette_id, role,
                    size_user_id, size_time, size
                )
                VALUES ($1, $2, $3, $4, NOW() AT TIME ZONE 'UTC', $5)
                ON CONFLICT (datacenter_id, roulette_id, role) DO UPDATE SET
                    size_user_id = EXCLUDED.size_user_id,
                    size_time = EXCLUDED.size_time,
                    size = EXCLUDED.size
                WHERE roulette_sizes.size_time < EXCLUDED.size_time"#r,
            DatabaseU16(datacenter_id).as_db(),
            DatabaseU16(u16::from(size_info.roulette_id)).as_db(),
            size_info.role.as_db() as DbRouletteRole,
            size_info.user_id,
            DatabaseU16(u16::from(u8::from(size))).as_db()
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(est_wait) = size_info.estimated_wait_time {
        sqlx::query!(
            r#"--sql
                INSERT INTO roulette_sizes
                (
                    datacenter_id, roulette_id, role,
                    est_time_user_id, est_time_time, est_time
                )
                VALUES ($1, $2, $3, $4, NOW() AT TIME ZONE 'UTC', $5)
                ON CONFLICT (datacenter_id, roulette_id, role) DO UPDATE SET
                    est_time_user_id = EXCLUDED.est_time_user_id,
                    est_time_time = EXCLUDED.est_time_time,
                    est_time = EXCLUDED.est_time
                WHERE roulette_sizes.est_time_time < EXCLUDED.est_time_time"#r,
            DatabaseU16(datacenter_id).as_db(),
            DatabaseU16(u16::from(size_info.roulette_id)).as_db(),
            size_info.role.as_db() as DbRouletteRole,
            size_info.user_id,
            DatabaseU16(u16::from(u8::from(est_wait))).as_db()
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}

// pub async fn refresh_roulette_estimates(pool: &PgPool) -> Result<PgQueryResult, Error> {
//     sqlx::query!(r#"REFRESH MATERIALIZED VIEW CONCURRENTLY roulette_estimates"#)
//         .execute(pool)
//         .await
// }

pub async fn get_roulette_estimates(pool: &PgPool) -> Result<Vec<RouletteEstimate>, Error> {
    sqlx::query_as!(
        DbRouletteEstimate,
        r#"--sql;
        SELECT
            datacenter_id, roulette_id,
            role AS "role: DbRouletteRole",
            GREATEST(size_time, est_time_time, wait_time_time) as "time: DatabaseDateTime",
            wait_time as duration, est_time as wait_time, size
        FROM roulette_sizes"#
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(RouletteEstimate::from).collect())
}

pub async fn get_roulette_estimates_by_datacenter_id(
    pool: &PgPool,
    datacenter_id: u16,
) -> Result<Vec<RouletteEstimate>, Error> {
    sqlx::query_as!(
        DbRouletteEstimate,
        r#"--sql;
        SELECT
            datacenter_id, roulette_id,
            role AS "role: DbRouletteRole",
            GREATEST(size_time, est_time_time, wait_time_time) as "time: DatabaseDateTime",
            wait_time as duration, est_time as wait_time, size
        FROM roulette_sizes
        WHERE datacenter_id = $1"#,
        DatabaseU16(datacenter_id).as_db()
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(RouletteEstimate::from).collect())
}

pub async fn get_roulette_estimates_by_datacenter_id_filtered(
    pool: &PgPool,
    datacenter_id: u16,
    roulette_ids: Vec<u8>,
) -> Result<Vec<RouletteEstimate>, Error> {
    let roulette_ids = roulette_ids
        .into_iter()
        .map(|id| DatabaseU16(id.into()).as_db())
        .collect::<Vec<_>>();
    sqlx::query_as!(
        DbRouletteEstimate,
        r#"--sql;
        SELECT
            datacenter_id, roulette_id,
            role AS "role: DbRouletteRole",
            GREATEST(size_time, est_time_time, wait_time_time) as "time: DatabaseDateTime",
            wait_time as duration, est_time as wait_time, size
        FROM roulette_sizes
        WHERE datacenter_id = $1 AND roulette_id = ANY($2)"#,
        DatabaseU16(datacenter_id).as_db(),
        roulette_ids.as_slice()
    )
    .fetch_all(pool)
    .await
    .map(|estimates| estimates.into_iter().map(RouletteEstimate::from).collect())
}
