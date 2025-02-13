use crate::storage::db::wrappers::DatabaseDateTime;
use sqlx::FromRow;

use super::duty::RecapUpdateType;

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "duty_update_type", rename_all = "snake_case")]
pub enum DbRecapUpdateType {
    Roulette = 0,
    Thd = 1,
    Players = 2,
    WaitTime = 3,
    None = 4,
}

impl From<Option<RecapUpdateType>> for DbRecapUpdateType {
    fn from(value: Option<RecapUpdateType>) -> Self {
        match value {
            Some(RecapUpdateType::Roulette) => DbRecapUpdateType::Roulette,
            Some(RecapUpdateType::Thd) => DbRecapUpdateType::Thd,
            Some(RecapUpdateType::Players) => DbRecapUpdateType::Players,
            Some(RecapUpdateType::WaitTime) => DbRecapUpdateType::WaitTime,
            None => DbRecapUpdateType::None,
        }
    }
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(type_name = "roulette_role", rename_all = "lowercase")]
pub enum DbRouletteRole {
    Tank = 1,
    Healer = 2,
    Dps = 3,
}

#[derive(Debug, FromRow)]
pub struct DbRouletteEstimate {
    pub datacenter_id: i16,
    pub roulette_id: i16,
    pub role: DbRouletteRole,

    pub time: Option<DatabaseDateTime>,
    pub duration: Option<f64>,
    pub size: Option<i16>,
    pub wait_time: Option<i16>,
}
