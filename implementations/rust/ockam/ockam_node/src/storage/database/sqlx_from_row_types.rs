use sqlx::database::HasValueRef;
use sqlx::error::BoxDynError;
use sqlx::postgres::any::AnyTypeInfoKind;
use sqlx::{Any, Database, Decode, Type, Value, ValueRef};

/// This type is used to map Option<T> fields for the types deriving `FrowRow`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nullable<T>(Option<T>);

impl<T: Clone> Nullable<T> {
    /// Return the Option corresponding to this value in the database
    pub fn to_option(&self) -> Option<T> {
        self.0.clone()
    }
}

impl<'d, T: Decode<'d, Any>> Decode<'d, Any> for Nullable<T> {
    fn decode(value: <Any as HasValueRef<'d>>::ValueRef) -> Result<Self, BoxDynError> {
        match value.type_info().kind() {
            AnyTypeInfoKind::Null => Ok(Nullable(None)),
            _ => Ok(Nullable(Some(<T as Decode<'d, Any>>::decode(value)?))),
        }
    }
}

impl<T: Type<Any>> Type<Any> for Nullable<T> {
    fn type_info() -> <Any as Database>::TypeInfo {
        <T as Type<Any>>::type_info()
    }

    fn compatible(ty: &<Any as Database>::TypeInfo) -> bool {
        <T as Type<Any>>::compatible(ty) || ty.kind() == AnyTypeInfoKind::Null
    }
}

/// This type is used to map boolean fields for the types deriving `FrowRow`.
/// Postgres provides a proper boolean type but SQLite maps them as integers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Boolean(bool);

impl Boolean {
    /// Return the bool value
    pub fn to_bool(&self) -> bool {
        self.0
    }
}

impl<'d> Decode<'d, Any> for Boolean {
    fn decode(value: <Any as HasValueRef<'d>>::ValueRef) -> Result<Self, BoxDynError> {
        match value.type_info().kind() {
            AnyTypeInfoKind::Bool => Ok(Boolean(sqlx::ValueRef::to_owned(&value).decode())),
            AnyTypeInfoKind::Integer => {
                let v: i64 = sqlx::ValueRef::to_owned(&value).decode();
                Ok(Boolean(v == 1))
            }
            other => Err(format!("expected BOOLEAN or INTEGER, got {:?}", other).into()),
        }
    }
}

impl Type<Any> for Boolean {
    fn type_info() -> <Any as Database>::TypeInfo {
        <bool as Type<Any>>::type_info()
    }

    fn compatible(ty: &<Any as Database>::TypeInfo) -> bool {
        <Boolean as Type<Any>>::type_info().kind() == ty.kind()
            || ty.kind() == AnyTypeInfoKind::Integer
    }
}
