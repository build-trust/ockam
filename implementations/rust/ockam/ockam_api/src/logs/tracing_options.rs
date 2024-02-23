use ockam_core::env::FromString;
use std::fmt::{Display, Formatter};

/// This data type specifies if tracing is enabled or not
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TracingEnabled {
    On,
    Off,
}

impl Display for TracingEnabled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TracingEnabled::On => f.write_str("on"),
            TracingEnabled::Off => f.write_str("off"),
        }
    }
}

impl FromString for TracingEnabled {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        FromString::from_string(s).map(|v| {
            if v {
                TracingEnabled::On
            } else {
                TracingEnabled::Off
            }
        })
    }
}
