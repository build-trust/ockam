use ockam_core::env::FromString;
use std::fmt::{Display, Formatter};

/// This data type specifies if tracing is enabled or not
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ExportingEnabled {
    On,
    Off,
}

impl Display for ExportingEnabled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportingEnabled::On => f.write_str("on"),
            ExportingEnabled::Off => f.write_str("off"),
        }
    }
}

impl FromString for ExportingEnabled {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        FromString::from_string(s).map(|v| {
            if v {
                ExportingEnabled::On
            } else {
                ExportingEnabled::Off
            }
        })
    }
}
