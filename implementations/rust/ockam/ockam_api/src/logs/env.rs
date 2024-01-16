use super::LogFormat;
use ockam_core::env::{get_env, get_env_with_default, FromString};
use std::fmt::{Display, Formatter};

pub fn log_level() -> Option<String> {
    get_env("OCKAM_LOG_LEVEL").unwrap_or_default()
}

pub fn logging_enabled() -> LoggingEnabled {
    get_env("OCKAM_LOGGING")
        .unwrap_or(Some(LoggingEnabled::Off))
        .unwrap_or(LoggingEnabled::Off)
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LoggingEnabled {
    On,
    Off,
}

impl Display for LoggingEnabled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggingEnabled::On => f.write_str("on"),
            LoggingEnabled::Off => f.write_str("off"),
        }
    }
}

impl FromString for LoggingEnabled {
    fn from_string(_s: &str) -> ockam_core::Result<Self> {
        Ok(LoggingEnabled::On)
    }
}

pub fn tracing_enabled() -> TracingEnabled {
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

pub fn log_max_size_bytes() -> u64 {
    let default = 100;
    get_env_with_default("OCKAM_LOG_MAX_SIZE_MB", default).unwrap_or(default) * 1024 * 1024
}

pub fn log_max_files() -> usize {
    let default: u64 = 60;
    get_env_with_default("OCKAM_LOG_MAX_FILES", default).unwrap_or(default) as usize
}

pub fn log_format() -> LogFormat {
    let default = LogFormat::Default;
    get_env_with_default("OCKAM_LOG_FORMAT", default.clone()).unwrap_or(default)
}
