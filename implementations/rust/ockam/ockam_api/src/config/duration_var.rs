use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::time::Duration;

use crate::Result;

/// This struct can be used to parse environment variables representing a Duration
pub struct DurationVar {
    pub duration: Duration,
}

impl DurationVar {
    pub fn new(duration: Duration) -> DurationVar {
        DurationVar { duration }
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

impl FromString for DurationVar {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(DurationVar {
            duration: parse_duration(s)?,
        })
    }
}

/// Parse a duration using a regular expression
pub fn parse_duration(arg: &str) -> Result<Duration> {
    let needles = DURATION_REGEX
        .get_or_init(|| {
            Regex::new(r"(?P<numeric_duration>[0-9]+)(?P<length_sigil>d|h|m|s|ms)?$").unwrap()
        })
        .captures(arg)
        .ok_or(ockam_core::Error::new(
            Origin::Api,
            Kind::Serialization,
            "Invalid duration.",
        ))?;
    let time = needles["numeric_duration"].parse::<u64>().map_err(|_| {
        ockam_core::Error::new(Origin::Api, Kind::Serialization, "Invalid duration.")
    })?;

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
mod tests {
    use proptest::prelude::*;
    use std::time::Duration;

    use super::*;

    const ONE_YEAR_MILLIS: u64 = 31_536_000_000;

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
