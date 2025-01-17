use crate::env::FromString;
use crate::errcode::{Kind, Origin};
use crate::{Error, Result};
use std::env;
use std::env::VarError;

/// Get environmental value `var_name`. If value is not found returns Ok(None)
pub fn get_env<T: FromString>(var_name: &str) -> Result<Option<T>> {
    match env::var(var_name) {
        Ok(val) => {
            match T::from_string(&val) {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(error(format!("The environment variable `{var_name}` cannot be decoded. The value `{val}` is invalid: {e:?}"))),
            }
        },
        Err(e) => match e {
            VarError::NotPresent => Ok(None),
            VarError::NotUnicode(_) => Err(error(format!("The environment variable `{var_name}` cannot be decoded because it is not some valid Unicode"))),
        },
    }
}

/// Get environmental value `var_name`. If value is not found returns None
pub fn get_env_ignore_error<T: FromString>(var_name: &str) -> Option<T> {
    get_env::<T>(var_name).ok().flatten()
}

/// Return true if `var_name` is set and has a valid value
pub fn is_set<T: FromString>(var_name: &str) -> Result<bool> {
    Ok(get_env::<T>(var_name)?.is_some())
}

/// Get environmental value `var_name`. If value is not found returns `default_value`
pub fn get_env_with_default<T: FromString>(var_name: &str, default_value: T) -> Result<T> {
    Ok(get_env::<T>(var_name)?.unwrap_or(default_value))
}

/// Get environmental value `var_name`. If value is not found returns `default_value`
pub fn get_env_with_default_ignore_error<T: FromString>(var_name: &str, default_value: T) -> T {
    get_env::<T>(var_name)
        .ok()
        .flatten()
        .unwrap_or(default_value)
}

pub(crate) fn error(msg: String) -> Error {
    Error::new(Origin::Core, Kind::Invalid, msg)
}
