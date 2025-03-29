use sqlx::any::AnyTypeInfoKind;
use sqlx::error::BoxDynError;
use sqlx::{Any, Database, Decode, Type, Value, ValueRef};
use sqlx_core::any::AnyValueRef;

/// This type is used to map Option<T> fields for the types deriving `FromRow`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nullable<T>(Option<T>);

impl<T> From<Option<T>> for Nullable<T> {
    fn from(value: Option<T>) -> Self {
        Nullable(value)
    }
}

impl<T> From<Nullable<T>> for Option<T> {
    fn from(value: Nullable<T>) -> Self {
        value.0
    }
}

impl<T: Clone> Nullable<T> {
    /// Create a new Nullable value
    pub fn new(t: Option<T>) -> Self {
        Nullable(t)
    }

    /// Return the Option corresponding to this value in the database
    pub fn to_option(&self) -> Option<T> {
        self.0.clone()
    }
}

impl<'d, T: Decode<'d, Any>> Decode<'d, Any> for Nullable<T> {
    fn decode(value: AnyValueRef<'d>) -> Result<Self, BoxDynError> {
        match value.type_info().kind() {
            AnyTypeInfoKind::Null => Ok(Nullable(None)),
            _ => Ok(Nullable(Some(T::decode(value)?))),
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

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        Boolean(value)
    }
}

impl From<Boolean> for bool {
    fn from(value: Boolean) -> Self {
        value.0
    }
}

impl Boolean {
    /// Create a new Boolean value
    pub fn new(b: bool) -> Self {
        Boolean(b)
    }

    /// Return the bool value
    pub fn to_bool(&self) -> bool {
        self.0
    }
}

impl<'d> Decode<'d, Any> for Boolean {
    fn decode(value: AnyValueRef<'d>) -> Result<Self, BoxDynError> {
        match value.type_info().kind() {
            AnyTypeInfoKind::Bool => Ok(Boolean(ValueRef::to_owned(&value).decode())),
            AnyTypeInfoKind::Integer => {
                let v: i64 = ValueRef::to_owned(&value).decode();
                Ok(Boolean(v == 1))
            }
            AnyTypeInfoKind::BigInt => {
                let v: i64 = ValueRef::to_owned(&value).decode();
                Ok(Boolean(v == 1))
            }
            other => Err(format!("expected BOOLEAN, INTEGER, or BIGINT, got {:?}", other).into()),
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
            || ty.kind() == AnyTypeInfoKind::BigInt
    }
}
