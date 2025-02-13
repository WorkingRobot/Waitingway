use crate::storage::db::wrappers::{DatabaseDateTime, DatabaseU64};
use duty::QueueLanguage;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod duty;
pub mod duty_db;
pub mod job_info;
pub mod login;
pub mod summary;
pub mod travel;
pub mod world_info;
pub mod world_status;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Connection {
    #[serde(skip)]
    pub user_id: Uuid,
    pub created_at: DatabaseDateTime,

    pub conn_user_id: DatabaseU64,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct WorldQueryFilter {
    pub world_id: Option<Vec<u16>>,
    pub datacenter_id: Option<Vec<u16>>,
    pub region_id: Option<Vec<u16>>,
}

#[derive(Debug, Deserialize)]
pub struct RouletteQueryFilter {
    pub roulette_id: Option<Vec<u8>>,
    pub lang: QueueLanguage,
}
