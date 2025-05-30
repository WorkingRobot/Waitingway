#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use serde::{Deserialize, Serialize};
use sqlx::error::BoxDynError;

macro_rules! define_unsigned_database_type {
    ($wrapper:ident, $unsigned:ty, $signed:ty) => {
        #[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
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
            fn decode(value: <D as sqlx::Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
                Ok($wrapper(<$signed>::decode(value)? as $unsigned))
            }
        }

        impl<'q, D: sqlx::Database> sqlx::Encode<'q, D> for $wrapper
        where
            $signed: sqlx::Encode<'q, D>,
        {
            fn encode_by_ref(
                &self,
                buf: &mut <D as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, BoxDynError> {
                (self.0 as $signed).encode_by_ref(buf)
            }
        }

        impl $wrapper {
            #[allow(dead_code)]
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

// define_unsigned_database_type!(DatabaseU8, u8, i8);
define_unsigned_database_type!(DatabaseU16, u16, i16);
define_unsigned_database_type!(DatabaseU32, u32, i32);
define_unsigned_database_type!(DatabaseU64, u64, i64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DatabaseDateTime(#[serde(with = "time::serde::rfc3339")] pub time::OffsetDateTime);

impl Default for DatabaseDateTime {
    fn default() -> Self {
        Self(time::OffsetDateTime::UNIX_EPOCH)
    }
}

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
    fn decode(value: <D as sqlx::Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(Self(time::PrimitiveDateTime::decode(value)?.assume_utc()))
    }
}

impl<'q, D: sqlx::Database> sqlx::Encode<'q, D> for DatabaseDateTime
where
    time::PrimitiveDateTime: sqlx::Encode<'q, D>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <D as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, BoxDynError> {
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
