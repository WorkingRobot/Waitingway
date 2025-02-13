use crate::storage::db::wrappers::DatabaseDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow)]
pub struct DbWorldSummaryInfo {
    pub world_id: Option<i16>,
    pub world_name: Option<String>,
    pub datacenter_id: Option<i16>,
    pub datacenter_name: Option<String>,
    pub region_id: Option<i16>,
    pub region_name: Option<String>,
    pub region_abbreviation: Option<String>,

    pub status: Option<i16>,
    pub category: Option<i16>,
    pub can_create: Option<bool>,

    pub prohibit: Option<bool>,

    pub time: Option<time::PrimitiveDateTime>,
    pub size: Option<i32>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSummaryInfo {
    pub world_id: u16,
    pub world_name: String,
    pub datacenter_id: u16,
    pub datacenter_name: String,
    pub region_id: u16,
    pub region_name: String,
    pub region_abbreviation: String,

    pub status: i16,
    pub category: i16,
    pub can_create: bool,

    pub travel_prohibit: bool,

    pub queue_time: DatabaseDateTime,
    pub queue_size: i32,
    pub queue_duration: f64,
}

impl From<DbWorldSummaryInfo> for WorldSummaryInfo {
    fn from(db: DbWorldSummaryInfo) -> Self {
        Self {
            world_id: db.world_id.unwrap_or_default() as u16,
            world_name: db.world_name.unwrap_or_default(),
            datacenter_id: db.datacenter_id.unwrap_or_default() as u16,
            datacenter_name: db.datacenter_name.unwrap_or_default(),
            region_id: db.region_id.unwrap_or_default() as u16,
            region_name: db.region_name.unwrap_or_default(),
            region_abbreviation: db.region_abbreviation.unwrap_or_default(),

            status: db.status.unwrap_or_default(),
            category: db.category.unwrap_or_default(),
            can_create: db.can_create.unwrap_or_default(),

            travel_prohibit: db.prohibit.unwrap_or_default(),

            queue_time: DatabaseDateTime::from(db.time.unwrap_or(time::PrimitiveDateTime::MIN)),
            queue_size: db.size.unwrap_or_default(),
            queue_duration: db.duration.unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Summary {
    pub average_travel_time: i32,
    pub worlds: Vec<WorldSummary>,
    pub datacenters: Vec<DatacenterSummary>,
    pub regions: Vec<RegionSummary>,
}

#[derive(Serialize, Deserialize)]
pub struct WorldSummary {
    pub id: u16,
    pub name: String,
    pub datacenter_id: u16,

    pub travel_prohibited: bool,
    pub world_status: i16,
    pub world_category: i16,
    pub world_character_creation_enabled: bool,

    pub queue_size: i32,
    pub queue_duration: f64,
    pub queue_last_update: DatabaseDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct DatacenterSummary {
    pub id: u16,
    pub name: String,
    pub region_id: u16,
    // pub lobby_ping: u32,
    // pub server_ping: u32,
    // pub packet_loss: f32,
    // pub open_ports: f32,
}

#[derive(Serialize, Deserialize)]
pub struct RegionSummary {
    pub id: u16,
    pub name: String,
    pub abbreviation: String,
}
