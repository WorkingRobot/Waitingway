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
    #[serde(with = "iso8601")]
    pub start_time: time::PrimitiveDateTime,
    // Time the queue was left
    #[serde(with = "iso8601")]
    pub end_time: time::PrimitiveDateTime,
    #[sqlx(skip)]
    pub positions: Vec<RecapPosition>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct RecapPosition {
    // Recap id that this position is for
    #[serde(skip)]
    pub recap_id: Uuid,

    // Time the position was updated
    #[serde(with = "iso8601")]
    pub time: time::PrimitiveDateTime,
    // Position of the player in the queue
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Connection {
    #[serde(skip)]
    pub user_id: Uuid,
    #[serde(with = "iso8601")]
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

pub mod iso8601 {
    use serde::de::Error as _;
    use serde::{Deserializer, Serializer};
    use time::{format_description::well_known::iso8601::Config, PrimitiveDateTime};
    use time::{
        format_description::well_known::{iso8601, Iso8601},
        macros::offset,
    };

    const CONFIG: iso8601::EncodedConfig = Config::DEFAULT.encode();
    const FORMAT: Iso8601<CONFIG> = Iso8601::<CONFIG>;

    time::serde::format_description!(my_format, OffsetDateTime, FORMAT);

    pub fn serialize<S: Serializer>(
        datetime: &PrimitiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        my_format::serialize(&datetime.assume_utc(), serializer)
    }

    pub fn deserialize<'a, D: Deserializer<'a>>(
        deserializer: D,
    ) -> Result<PrimitiveDateTime, D::Error> {
        my_format::deserialize(deserializer).and_then(|datetime| {
            let datetime = datetime
                .checked_to_offset(offset!(UTC))
                .ok_or(D::Error::custom("Invalid datetime"))?;
            Ok(PrimitiveDateTime::new(datetime.date(), datetime.time()))
        })
    }
}
