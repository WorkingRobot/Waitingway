use crate::{
    db_wrappers::{DatabaseDateTime, DatabaseU16, DatabaseU64},
    middleware::version::UserAgentVersion,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Recap {
    // Unique id of the recap
    #[serde(skip)]
    pub id: Uuid,
    // User/player id that the recap was from
    #[serde(skip)]
    pub user_id: Uuid,
    // World id that the recap was for
    pub world_id: DatabaseU16,
    // Whether the user has a free trial and may be deprioritized (Can they even join a queue?)
    pub free_trial: bool,
    // Whether the queue was successful or not (false = manual disconnect, true = successful queue & login)
    pub successful: bool,
    // Whether the player reentered the queue after a disconnect/cancellation
    pub reentered: bool,
    // Error info if the queue was not successful. May indicate a server error. Will be used for viewing error rates.
    #[sqlx(flatten)]
    pub error: Option<RecapError>,
    // Time the queue was started
    pub start_time: DatabaseDateTime,
    // Time the queue was left
    pub end_time: DatabaseDateTime,
    // Time the client sent an identify request for the end of the queue
    pub end_identify_time: Option<DatabaseDateTime>,
    #[sqlx(skip)]
    pub positions: Vec<RecapPosition>,
    #[sqlx(skip)]
    #[serde(skip)]
    pub client_version: UserAgentVersion,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct RecapPosition {
    // Recap id that this position is for
    #[serde(skip)]
    pub recap_id: Uuid,

    // Time the position was updated
    pub time: DatabaseDateTime,
    // Time the client sent an identify request for this update
    pub identify_time: Option<DatabaseDateTime>,
    // Position of the player in the queue
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RecapError {
    #[sqlx(rename = "error_type")]
    pub r#type: i32,
    #[sqlx(rename = "error_code")]
    pub code: i32,
    #[sqlx(rename = "error_info")]
    pub info: String,
    pub error_row: DatabaseU16,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct QueueSize {
    // User/player id that the size update was from
    #[serde(skip)]
    pub user_id: Uuid,
    // World id that the queue size is for
    pub world_id: DatabaseU16,
    // Size of the queue
    pub size: i32,
}

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

// sqlx throws a fit and doesn't accept sqlx(rename = "") nor Option<DatabaseU16>
#[derive(Debug, sqlx::FromRow)]
pub struct DbQueueEstimate {
    pub world_id: Option<i16>,

    pub time: Option<time::PrimitiveDateTime>,
    pub size: Option<i32>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEstimate {
    pub world_id: u16,

    pub last_update: DatabaseDateTime,
    pub last_size: i32,
    pub last_duration: f64,
}

impl From<DbQueueEstimate> for QueueEstimate {
    fn from(db: DbQueueEstimate) -> Self {
        Self {
            world_id: db.world_id.unwrap_or_default() as u16,
            last_update: DatabaseDateTime::from(db.time.unwrap_or(time::PrimitiveDateTime::MIN)),
            last_size: db.size.unwrap_or_default(),
            last_duration: db.duration.unwrap_or_default(),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DbTravelState {
    pub world_id: i16,
    pub prohibit: bool,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelResponse {
    pub error: Option<String>,
    pub result: DCTravelResult,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelResult {
    #[serde(rename = "return_code")]
    pub code: String,
    #[serde(rename = "return_status")]
    pub status: String,
    #[serde(rename = "return_errcode")]
    pub errcode: String,

    pub data: Option<DCTravelData>,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelData {
    #[serde(rename = "homeDC")]
    #[allow(dead_code)]
    pub home_dc: u8,
    #[serde(rename = "homeWorldId")]
    pub home_world_id: u16,
    #[serde(rename = "worldInfos")]
    pub datacenters: Vec<DCTravelDCInfo>,
    #[serde(rename = "averageElapsedTime")]
    pub average_elapsed_time: i32,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelDCInfo {
    #[allow(dead_code)]
    pub dc: u8,
    #[serde(rename = "worldIds")]
    pub worlds: Vec<DCTravelWorldInfo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DCTravelWorldInfo {
    pub id: u16,
    #[serde(rename = "travelFlag")]
    pub travel: u8,
    #[serde(rename = "acceptFlag")]
    pub accept: u8,
    #[serde(rename = "prohibitFlag")]
    pub prohibit: u8,
}

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
