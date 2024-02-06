use ockam_core::env::FromString;
use ockam_core::errcode::{Kind, Origin};
use std::fmt::{Display, Formatter};

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum GlobalErrorHandler {
    Off,
    Console,
    LogFile,
}

impl Display for GlobalErrorHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalErrorHandler::Off => f.write_str("off"),
            GlobalErrorHandler::Console => f.write_str("console"),
            GlobalErrorHandler::LogFile => f.write_str("logfile"),
        }
    }
}

impl FromString for GlobalErrorHandler {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "off" => Ok(GlobalErrorHandler::Off),
            "console" => Ok(GlobalErrorHandler::Console),
            "logfile" => Ok(GlobalErrorHandler::LogFile),
            _ => Err(ockam_core::Error::new(
                Origin::Api,
                Kind::Serialization,
                format!("incorrect value for the global error handler {s}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Colored {
    On,
    Off,
}

/// Options for selecting the log format used in files or in the console
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Default,
    Pretty,
    Json,
}

impl FromString for LogFormat {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "pretty" => Ok(LogFormat::Pretty),
            "json" => Ok(LogFormat::Json),
            _ => Ok(LogFormat::Default),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogFormat::Default => write!(f, "default"),
            LogFormat::Pretty => write!(f, "pretty"),
            LogFormat::Json => write!(f, "json"),
        }
    }
}
