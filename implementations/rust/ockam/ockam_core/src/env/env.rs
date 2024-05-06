use crate::env::FromString;
use crate::errcode::{Kind, Origin};
use crate::{Error, Result};
use std::env;
use std::env::VarError;

/// Get environmental value `var_name`. If value is not found returns Ok(None)
pub fn get_env<T: FromString>(var_name: &str) -> Result<Option<T>> {
    get_env_impl::<Option<T>>(var_name, None)
}

/// Return true if `var_name` is set and has a valid value
pub fn is_set<T: FromString>(var_name: &str) -> Result<bool> {
    get_env_impl::<Option<T>>(var_name, None).map(|v| v.is_some())
}

/// Get environmental value `var_name`. If value is not found returns `default_value`
pub fn get_env_with_default<T: FromString>(var_name: &str, default_value: T) -> Result<T> {
    get_env_impl::<T>(var_name, default_value)
}

fn get_env_impl<T: FromString>(var_name: &str, default_value: T) -> Result<T> {
    match env::var(var_name) {
        Ok(val) => {
            match T::from_string(&val) {
                Ok(v) => Ok(v),
                Err(e) => Err(error(format!("The environment variable `{var_name}` cannot be decoded. The value `{val}` is invalid: {e:?}"))),
            }
        },
        Err(e) => match e {
            VarError::NotPresent => Ok(default_value),
            VarError::NotUnicode(_) => Err(error(format!("The environment variable `{var_name}` cannot be decoded because it is not some valid Unicode"))),
        },
    }
}

pub(crate) fn error(msg: String) -> Error {
    Error::new(Origin::Core, Kind::Invalid, msg)
}
