use crate::{
    models::world_info::{DbWorldInfo, WorldInfo},
    storage::db::wrappers::DatabaseU16,
};
use sqlx::{Error, PgPool, QueryBuilder};

pub async fn get_worlds(pool: &PgPool) -> Result<Vec<WorldInfo>, Error> {
    sqlx::query_as!(DbWorldInfo, r#"SELECT * FROM worlds"#)
        .fetch_all(pool)
        .await
        .map(|worlds| worlds.into_iter().map(WorldInfo::from).collect())
}

pub async fn upsert_worlds(pool: &PgPool, worlds: Vec<WorldInfo>) -> Result<(), Error> {
    let mut query_builder = QueryBuilder::new(
        r#"--sql;
            INSERT INTO worlds (world_id, world_name, datacenter_id, datacenter_name, region_id, region_name, region_abbreviation, is_cloud, hidden)
            "#,
    );
    query_builder.push_values(worlds, |mut b, world| {
        b.push_bind(DatabaseU16(world.world_id).as_db())
            .push_bind(world.world_name)
            .push_bind(DatabaseU16(world.datacenter_id).as_db())
            .push_bind(world.datacenter_name)
            .push_bind(DatabaseU16(world.region_id).as_db())
            .push_bind(world.region_name)
            .push_bind(world.region_abbreviation)
            .push_bind(world.is_cloud)
            .push_bind(world.hidden);
    });
    query_builder.push(
        r#"
        ON CONFLICT (world_id) DO UPDATE
            SET world_name = EXCLUDED.world_name,
                datacenter_id = EXCLUDED.datacenter_id,
                datacenter_name = EXCLUDED.datacenter_name,
                region_id = EXCLUDED.region_id,
                region_name = EXCLUDED.region_name,
                region_abbreviation = EXCLUDED.region_abbreviation,
                is_cloud = EXCLUDED.is_cloud,
                hidden = EXCLUDED.hidden"#,
    );
    query_builder.build().execute(pool).await?;

    Ok(())
}
