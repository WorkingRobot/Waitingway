use crate::middleware::version::UserAgentVersion;
use serde::{Deserialize, Serialize};
use sqlx::{database::HasValueRef, error::BoxDynError, FromRow};
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

macro_rules! define_unsigned_database_type {
    ($wrapper:ident, $unsigned:ty, $signed:ty) => {
        #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
        pub struct $wrapper(pub $unsigned);

        impl<D: sqlx::Database> sqlx::Type<D> for $wrapper
        where
            $signed: sqlx::Type<D>,
        {
            fn type_info() -> D::TypeInfo {
                <$signed as sqlx::Type<D>>::type_info()
            }
        }

        impl<'r, D: sqlx::Database> sqlx::Decode<'r, D> for $wrapper
        where
            $signed: sqlx::Decode<'r, D>,
        {
            fn decode(value: <D as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
                Ok($wrapper(<$signed>::decode(value)? as $unsigned))
            }
        }

        impl<'q, D: sqlx::Database> sqlx::Encode<'q, D> for $wrapper
        where
            $signed: sqlx::Encode<'q, D>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <D as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
            ) -> sqlx::encode::IsNull {
                (self.0 as $signed).encode_by_ref(buf)
            }
        }

        impl $wrapper {
            #[inline]
            pub fn as_db(self) -> $signed {
                self.0 as $signed
            }
        }

        impl From<$signed> for $wrapper {
            #[inline]
            fn from(value: $signed) -> Self {
                Self(value as $unsigned)
            }
        }

        impl From<$unsigned> for $wrapper {
            #[inline]
            fn from(value: $unsigned) -> Self {
                Self(value)
            }
        }
    };
}

define_unsigned_database_type!(DatabaseU16, u16, i16);
// define_unsigned_database_type!(DatabaseU32, u32, i32);
define_unsigned_database_type!(DatabaseU64, u64, i64);

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DatabaseDateTime(#[serde(with = "time::serde::rfc3339")] pub time::OffsetDateTime);

impl<D: sqlx::Database> sqlx::Type<D> for DatabaseDateTime
where
    time::PrimitiveDateTime: sqlx::Type<D>,
{
    fn type_info() -> D::TypeInfo {
        <time::PrimitiveDateTime as sqlx::Type<D>>::type_info()
    }
}

impl<'r, D: sqlx::Database> sqlx::Decode<'r, D> for DatabaseDateTime
where
    time::PrimitiveDateTime: sqlx::Decode<'r, D>,
{
    fn decode(value: <D as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
        Ok(Self(time::PrimitiveDateTime::decode(value)?.assume_utc()))
    }
}

impl<'q, D: sqlx::Database> sqlx::Encode<'q, D> for DatabaseDateTime
where
    time::PrimitiveDateTime: sqlx::Encode<'q, D>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <D as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        self.as_db().encode_by_ref(buf)
    }
}

impl DatabaseDateTime {
    #[inline]
    pub fn as_db(self) -> time::PrimitiveDateTime {
        let time = self
            .0
            .checked_to_offset(time::UtcOffset::UTC)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        time::PrimitiveDateTime::new(time.date(), time.time())
    }
}

impl From<time::PrimitiveDateTime> for DatabaseDateTime {
    #[inline]
    fn from(value: time::PrimitiveDateTime) -> Self {
        Self(value.assume_utc())
    }
}

impl From<time::OffsetDateTime> for DatabaseDateTime {
    #[inline]
    fn from(value: time::OffsetDateTime) -> Self {
        Self(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueEstimate {
    pub world_id: u16,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,

    pub conn_user_id: DatabaseU64,
    pub username: String,
    pub display_name: String,
}
