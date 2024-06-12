use chrono::{DateTime, Utc};
use sqlx::database::HasArguments;
use sqlx::encode::IsNull;
use sqlx::{Any, Database, Encode, Type};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use ockam_core::Address;

/// This enum represents the set of types that we currently support in our database.
/// We support the types which Sqlite uses:
/// https://www.sqlite.org/datatype3.html
///
/// The purpose of this type is to ease the serialization of data types in Ockam into data types in
/// our database. For example, if we describe how to translate an `Identifier` into some `Text` then
/// we can use the `Text` as a parameter in a sqlx query.
///
/// Note: see the `ToSqlxType` trait and its instances for how the conversion is done
///
pub enum SqlxType {
    /// This type represents text in the database
    Text(String),
    /// This type represents arbitrary bytes in the database
    Blob(Vec<u8>),
    /// This type represents ints, signed or unsigned
    Integer(i64),
    /// This type represents floats
    #[allow(unused)]
    Real(f64),
}

/// The SqlxType implements the Type<Any> trait from sqlx to allow its values to be serialized
/// to any database
impl Type<Any> for SqlxType {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

/// The SqlType implements the Encode<Any> trait from sqlx to allow its values to be serialized
/// to any database. There is a 1 to 1 mapping with the database native types
impl Encode<'_, Any> for SqlxType {
    fn encode_by_ref(&self, buf: &mut <Any as HasArguments>::ArgumentBuffer) -> IsNull {
        match self {
            SqlxType::Text(v) => <String as Encode<'_, Any>>::encode_by_ref(v, buf),
            SqlxType::Blob(v) => <Vec<u8> as Encode<'_, Any>>::encode_by_ref(v, buf),
            SqlxType::Integer(v) => <i64 as Encode<'_, Any>>::encode_by_ref(v, buf),
            SqlxType::Real(v) => <f64 as Encode<'_, Any>>::encode_by_ref(v, buf),
        }
    }

    fn produces(&self) -> Option<<Any as Database>::TypeInfo> {
        Some(match self {
            SqlxType::Text(_) => <String as Type<Any>>::type_info(),
            SqlxType::Blob(_) => <Vec<u8> as Type<Any>>::type_info(),
            SqlxType::Integer(_) => <i64 as Type<Any>>::type_info(),
            SqlxType::Real(_) => <f64 as Type<Any>>::type_info(),
        })
    }
}

/// This trait can be implemented by any type that can be converted to a database type
/// Typically an `Identifier` (to a `Text`), a `TimestampInSeconds` (to an `Integer`) etc...
///
/// This allows a value to be used as a bind parameters in a sqlx query for example:
///
/// use std::str::FromStr;
/// use sqlx::query_as;
/// use ockam_node::database::{SqlxType, ToSqlxType};
///
/// // newtype for a UNIX-like timestamp
/// struct TimestampInSeconds(u64);
///
/// // this implementation maps the TimestampInSecond type to one of the types that Sqlx
/// // can serialize for any database
/// impl ToSqlxType for TimestampInSeconds {
///     fn to_sql(&self) -> SqlxType {
///         self.0.to_sql()
///     }
/// }
///
/// let timestamp = TimestampInSeconds(10000000);
/// let query = query_as("SELECT identifier, change_history FROM identity WHERE created_at >= $1").bind(timestamp.as_sql());
///
///
pub trait ToSqlxType {
    /// Return the appropriate sql type
    fn to_sql(&self) -> SqlxType;
}

impl ToSqlxType for u64 {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(*self as i64)
    }
}

impl ToSqlxType for u32 {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(*self as i64)
    }
}

impl ToSqlxType for u16 {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(*self as i64)
    }
}

impl ToSqlxType for u8 {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(*self as i64)
    }
}

impl ToSqlxType for i8 {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(*self as i64)
    }
}

impl ToSqlxType for OffsetDateTime {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Integer(self.unix_timestamp())
    }
}

impl ToSqlxType for DateTime<Utc> {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.to_rfc3339())
    }
}

impl ToSqlxType for &[u8; 32] {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Blob(self.to_vec().clone())
    }
}

impl ToSqlxType for SocketAddr {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.to_string())
    }
}

impl ToSqlxType for Address {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(self.to_string())
    }
}

impl ToSqlxType for PathBuf {
    fn to_sql(&self) -> SqlxType {
        self.as_path().to_sql()
    }
}

impl ToSqlxType for &Path {
    fn to_sql(&self) -> SqlxType {
        SqlxType::Text(
            self.to_str()
                .unwrap_or("a path should be a valid string")
                .into(),
        )
    }
}
