use crate::storage::db::wrappers::DatabaseU16;
use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct DbWorldStatus {
    pub world_id: DatabaseU16,
    pub status: i16,
    pub category: i16,
    #[serde(rename = "create")]
    pub can_create: bool,
}

#[derive(Debug, Deserialize)]
pub struct WorldStatusResponse {
    pub data: Vec<WorldStatusRegionInfo>,
}

#[derive(Debug, Deserialize)]
pub struct WorldStatusRegionInfo {
    #[allow(dead_code)]
    pub name: String,
    pub dc: Vec<WorldStatusDCInfo>,
}

#[derive(Debug, Deserialize)]
pub struct WorldStatusDCInfo {
    #[allow(dead_code)]
    pub name: String,
    pub world: Vec<WorldStatusWorldInfo>,
}

#[derive(Debug, Deserialize)]
pub struct WorldStatusWorldInfo {
    pub name: String,
    pub status: i16,
    pub category: i16,
    pub create: bool,
}
