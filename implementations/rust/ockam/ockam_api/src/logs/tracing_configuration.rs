use ockam_core::env::{get_env, FromString};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracingConfiguration {
    enabled: TracingEnabled,
}

impl TracingConfiguration {
    pub fn is_enabled(&self) -> bool {
        self.enabled == TracingEnabled::On
    }
}

impl Display for TracingConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("tracing")
            .field("enabled", &self.enabled.to_string())
            .finish()
    }
}

pub fn tracing_configuration() -> TracingConfiguration {
    TracingConfiguration {
        enabled: tracing_enabled(),
    }
}

pub(crate) fn tracing_enabled() -> TracingEnabled {
    get_env("OCKAM_TRACING")
        .unwrap_or(Some(TracingEnabled::Off))
        .unwrap_or(TracingEnabled::Off)
}

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
    fn from_string(_s: &str) -> ockam_core::Result<Self> {
        Ok(TracingEnabled::On)
    }
}
