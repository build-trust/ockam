use crate::env::error;
use crate::errcode::{Kind, Origin};
use crate::{Error, Result};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::path::PathBuf;
use std::time::Duration;

/// For-internal-use trait for types that can be parsed from string
pub trait FromString: Sized {
    /// Parses string and gives the result. Can return an error in case
    /// of parsing error.
    fn from_string(s: &str) -> Result<Self>;
}

/// Instances

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

/// Regex for durations: (?P<numeric_duration>[0-9]+)(?P<length_sigil>d|h|m|s|ms)?$
/// It accepts a number and a unit:
///
///  - h: hour
///  - m: minute
///  - s: second
///  - ms: millisecond
///
/// For example: 1ms, 2s, 10m
static DURATION_REGEX: OnceCell<Regex> = OnceCell::new();

impl FromString for Duration {
    fn from_string(s: &str) -> Result<Self> {
        parse_duration(s)
    }
}

/// Parse a duration using a regular expression. This function can be reused to parse arguments
pub fn parse_duration(arg: &str) -> Result<Duration> {
    let needles = DURATION_REGEX
        .get_or_init(|| {
            Regex::new(r"(?P<numeric_duration>[0-9]+)(?P<length_sigil>d|h|m|s|ms)?$").unwrap()
        })
        .captures(arg)
        .ok_or_else(|| Error::new(Origin::Api, Kind::Serialization, "Invalid duration."))?;
    let time = needles["numeric_duration"]
        .parse::<u64>()
        .map_err(|_| Error::new(Origin::Api, Kind::Serialization, "Invalid duration."))?;

    match needles.name("length_sigil") {
        Some(n) => match n.as_str() {
            "ms" => Ok(Duration::from_millis(time)),
            "s" => Ok(Duration::from_secs(time)),
            "m" => Ok(Duration::from_secs(60 * time)),
            "h" => Ok(Duration::from_secs(60 * 60 * time)),
            "d" => Ok(Duration::from_secs(60 * 60 * 24 * time)),
            _ => unreachable!("Alternatives excluded by regex."),
        },
        None => Ok(Duration::from_secs(time)),
    }
}

#[cfg(test)]
mod tests_from_string_trait {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;

    const ONE_YEAR_MILLIS: u64 = 31_536_000_000;

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

    proptest! {
        #[test]
        fn test_reverse_duration(arg in 0..ONE_YEAR_MILLIS) {
            let millis = Duration::from_millis(arg);
            let milli_str = format!("{}ms", millis.as_millis());
            prop_assert_eq!(parse_duration(milli_str.as_str()).unwrap(), millis);

            // We need to truncate the value before calculating seconds,
            // so pass it through Duration
            let secs = Duration::from_secs(millis.as_secs());
            let secs_str_s = format!("{}s", secs.as_secs());
            let secs_str = format!("{}", secs.as_secs());
            prop_assert_eq!(parse_duration(secs_str_s.as_str()).unwrap(), secs);
            prop_assert_eq!(parse_duration(secs_str.as_str()).unwrap(), secs);

            let mins = Duration::from_secs(secs.as_secs() / 60 * 60);
            let mins_str = format!("{}m", mins.as_secs() / 60);
            prop_assert_eq!(parse_duration(mins_str.as_str()).unwrap(), mins);

            let hrs = Duration::from_secs(secs.as_secs() / 60 * 60 * 60);
            let hrs_str = format!("{}h", hrs.as_secs() / 60 / 60);
            prop_assert_eq!(parse_duration(hrs_str.as_str()).unwrap(), hrs);

            let days = Duration::from_secs(secs.as_secs() / 60 * 60 * 60 * 24);
            let days_str = format!("{}d", days.as_secs() / 60 / 60 / 24);
            prop_assert_eq!(parse_duration(days_str.as_str()).unwrap(), days)
        }
    }
}
