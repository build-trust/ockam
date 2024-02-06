use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use std::str::FromStr;
use tracing_core::Level;

/// This struct can be used to parse environment variables representing a log level
pub struct LevelVar {
    pub level: Level,
}

impl FromString for LevelVar {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        Ok(LevelVar {
            level: Level::from_str(s)
                .map_err(|e| Error::new(Origin::Api, Kind::Serialization, format!("{e:?}")))?,
        })
    }
}
