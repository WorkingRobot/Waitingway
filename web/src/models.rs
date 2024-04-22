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

    pub conn_user_id: DatabaseU64,
    pub username: String,
    pub display_name: String,
}

macro_rules! define_unsigned_database_type {
    ($wrapper:ident, $unsigned:ty, $signed:ty) => {
        #[derive(Debug, Serialize, Deserialize)]
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
            pub fn as_db(&self) -> $signed {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueEstimate {
    pub world_id: u16,
    #[serde(with = "iso8601")]
    pub created_at: time::PrimitiveDateTime,

    pub conn_user_id: DatabaseU64,
    pub username: String,
    pub display_name: String,
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
            to_utc_primitive(datetime).ok_or(D::Error::custom("Invalid datetime"))
        })
    }

    pub const fn to_utc_primitive(
        datetime: time::OffsetDateTime,
    ) -> Option<time::PrimitiveDateTime> {
        konst::option::map!(datetime.checked_to_offset(offset!(UTC)), |datetime| {
            time::PrimitiveDateTime::new(datetime.date(), datetime.time())
        })
    }
}
