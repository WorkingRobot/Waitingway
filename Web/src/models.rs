use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Recap {
    // Unique id of the recap
    pub id: Uuid,
    // User/player id that the recap was from
    pub user_id: Uuid,
    // World id that the recap was for
    pub world_id: i16,
    // Whether the queue was successful or not (false = manual disconnect, true = successful queue & login)
    pub successful: bool,
    // Time the queue was started
    pub start_time: time::PrimitiveDateTime,
    // Time the queue was left
    pub end_time: time::PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RecapPosition {
    // Recap id that this position is for
    pub recap_id: Uuid,

    // Time the position was updated
    pub time: time::PrimitiveDateTime,
    // Position of the player in the queue
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Connection {
    pub user_id: Uuid,
    pub created_at: time::PrimitiveDateTime,

    #[sqlx(try_from = "i64")]
    pub conn_user_id: DiscordId,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordId(pub u64);

impl From<i64> for DiscordId {
    fn from(value: i64) -> Self {
        DiscordId(value as u64)
    }
}

impl DiscordId {
    pub fn as_db(&self) -> i64 {
        self.0 as i64
    }
}
