use crate::alloc::borrow::ToOwned;
use crate::alloc::string::ToString;
use crate::compat::fmt::Vec;
use crate::compat::string::String;
use crate::errcode::{Kind, Origin};
use crate::{Error, Result};
#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::env::VarError;
#[cfg(feature = "std")]
use std::path::PathBuf;

/// Get environmental value [var_name]. If value is not found returns Ok(None)
#[cfg(feature = "std")]
pub fn get_env<T: FromString>(var_name: &str) -> Result<Option<T>> {
    get_env_impl::<Option<T>>(var_name, None)
}

/// Get environmental value [var_name]. If value is not found returns [default_value]
#[cfg(feature = "std")]
pub fn get_env_with_default<T: FromString>(var_name: &str, default_value: T) -> Result<T> {
    get_env_impl::<T>(var_name, default_value)
}

#[cfg(feature = "std")]
fn get_env_impl<T: FromString>(var_name: &str, default_value: T) -> Result<T> {
    match env::var(var_name) {
        Ok(val) => Ok(T::from_string(&val)?),
        Err(e) => match e {
            VarError::NotPresent => Ok(default_value),
            VarError::NotUnicode(_) => Err(error("get_env error: not unicode".to_owned())),
        },
    }
}

/// For-internal-use trait for types that can be parsed from string
pub trait FromString: Sized {
    /// Parses string and gives the result. Can return an error in case
    /// of parsing error.
    fn from_string(s: &str) -> Result<Self>;
}

impl<T: FromString> FromString for Option<T> {
    fn from_string(s: &str) -> Result<Self> {
        let result = T::from_string(s);
        if let Ok(val) = result {
            return Ok(Some(val));
        }
        Err(result.err().unwrap())
    }
}

impl FromString for bool {
    fn from_string(s: &str) -> Result<Self> {
        let s = s.to_lowercase();
        match s.as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(error(format!("bool parsing error: {}", s))),
        }
    }
}

impl FromString for char {
    fn from_string(s: &str) -> Result<Self> {
        if s.len() != 1 {
            return Err(error("char parsing error: length != 1".to_owned()));
        }

        Ok(s.chars().next().unwrap())
    }
}

impl FromString for String {
    fn from_string(s: &str) -> Result<Self> {
        Ok(s.to_owned())
    }
}

impl<T: FromString> FromString for Vec<T> {
    fn from_string(s: &str) -> Result<Self> {
        s.split(',').map(|x| T::from_string(x)).collect()
    }
}

impl FromString for u8 {
    fn from_string(s: &str) -> Result<Self> {
        s.parse::<u8>()
            .map_err(|_| error("u8 parsing error".to_string()))
    }
}

impl FromString for u16 {
    fn from_string(s: &str) -> Result<Self> {
        s.parse::<u16>()
            .map_err(|_| error("u16 parsing error".to_string()))
    }
}

impl FromString for u32 {
    fn from_string(s: &str) -> Result<Self> {
        s.parse::<u32>()
            .map_err(|_| error("u32 parsing error".to_string()))
    }
}

impl FromString for u64 {
    fn from_string(s: &str) -> Result<Self> {
        s.parse::<u64>()
            .map_err(|_| error("u64 parsing error".to_string()))
    }
}

#[cfg(feature = "std")]
impl FromString for PathBuf {
    fn from_string(s: &str) -> Result<Self> {
        Ok(PathBuf::from(&s))
    }
}

fn error(msg: String) -> Error {
    Error::new(Origin::Core, Kind::Internal, msg)
}

#[cfg(test)]
mod tests_from_string_trait {
    use super::*;

    #[test]
    fn test_bool_ok() {
        bool::from_string("true").unwrap();
        bool::from_string("false").unwrap();
        bool::from_string("TRUE").unwrap();
        bool::from_string("FALSE").unwrap();
        bool::from_string("tRuE").unwrap();
        bool::from_string("fAlSe").unwrap();
        bool::from_string("0").unwrap();
        bool::from_string("1").unwrap();
        bool::from_string("yes").unwrap();
        bool::from_string("no").unwrap();
    }

    #[test]
    fn test_bool_err() {
        assert!(bool::from_string("something").is_err());
        assert!(bool::from_string("").is_err());
    }

    #[test]
    fn test_string_ok() {
        assert_eq!(
            "something".to_owned(),
            String::from_string("something").unwrap()
        );
        assert_eq!("".to_owned(), String::from_string("").unwrap());
    }

    #[test]
    fn test_vec_ok() {
        Vec::<bool>::from_string("1,0,TRUE,FALSE,true,fAlSe").unwrap();
        Vec::<char>::from_string("a,b,c,d,e").unwrap();
        Vec::<String>::from_string("hello,world").unwrap();
    }

    #[test]
    fn test_vec_err() {
        assert!(Vec::<bool>::from_string("").is_err());
        assert!(Vec::<u8>::from_string("1,2,3,100000").is_err());
        assert!(Vec::<u8>::from_string("1, 2").is_err());
        assert!(Vec::<u8>::from_string("1,").is_err())
    }

    #[test]
    fn test_ints_ok() {
        u8::from_string("1").unwrap();
        u16::from_string("65535").unwrap();
        u32::from_string("4294967295").unwrap();
        u64::from_string("18446744073709551615").unwrap();
    }

    #[test]
    fn test_ints_err() {
        assert!(u8::from_string("-1").is_err());
        assert!(u8::from_string("256").is_err());
        assert!(u16::from_string("65536").is_err());
        assert!(u32::from_string("4294967296").is_err());
        assert!(u64::from_string("18446744073709551616").is_err());
    }

    #[test]
    fn test_option_ok() {
        let result = Option::<u8>::from_string("1");
        assert_eq!(result.unwrap(), Some(1));
    }
}
