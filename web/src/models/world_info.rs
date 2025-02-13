use crate::storage::db::wrappers::DatabaseU16;
use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct DbWorldInfo {
    pub world_id: DatabaseU16,
    pub world_name: String,
    pub datacenter_id: DatabaseU16,
    pub datacenter_name: String,
    pub region_id: DatabaseU16,
    pub region_name: String,
    pub region_abbreviation: String,
    pub is_cloud: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub world_id: u16,
    pub world_name: String,
    pub datacenter_id: u16,
    pub datacenter_name: String,
    pub region_id: u16,
    pub region_name: String,
    pub region_abbreviation: String,
    pub is_cloud: bool,
    pub hidden: bool,
}

impl From<DbWorldInfo> for WorldInfo {
    fn from(db: DbWorldInfo) -> Self {
        Self {
            world_id: db.world_id.0,
            world_name: db.world_name,
            datacenter_id: db.datacenter_id.0,
            datacenter_name: db.datacenter_name,
            region_id: db.region_id.0,
            region_name: db.region_name,
            region_abbreviation: db.region_abbreviation,
            is_cloud: db.is_cloud,
            hidden: db.hidden,
        }
    }
}
