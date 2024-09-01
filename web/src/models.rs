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
pub struct QueueQueryFilter {
    pub world_id: Option<u16>,
    pub datacenter_id: Option<u16>,
    pub region_id: Option<u16>,
}

// sqlx throws a fit and doesn't accept sqlx(rename = "") nor Option<DatabaseU16>
#[derive(Debug, sqlx::FromRow)]
pub struct DbQueueEstimate {
    pub world_id: Option<i16>,

    pub time: Option<time::PrimitiveDateTime>,
    pub size: Option<i32>,
    pub duration: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
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
