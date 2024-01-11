use super::{tracing_enabled, LogFormat, TracingEnabled};
use ockam_core::env::{get_env, get_env_with_default, FromString};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use tracing_core::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfiguration {
    level: LevelFilter,
    enabled: LoggingEnabled,
    max_size_bytes: u64,
    max_files: usize,
    format: LogFormat,
    colored: Colored,
    log_dir: Option<PathBuf>,
    crates: Vec<String>,
}

impl LoggingConfiguration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: LevelFilter,
        enabled: LoggingEnabled,
        max_size_bytes: u64,
        max_files: usize,
        format: LogFormat,
        colored: Colored,
        log_dir: Option<PathBuf>,
        crates: &[&str],
    ) -> LoggingConfiguration {
        LoggingConfiguration {
            level,
            enabled,
            max_size_bytes,
            max_files,
            format,
            colored,
            log_dir,
            crates: crates.iter().map(|c| c.to_string()).collect(),
        }
    }

    pub fn level(&self) -> LevelFilter {
        self.level
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled == LoggingEnabled::On
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_size_bytes
    }

    pub fn max_files(&self) -> usize {
        self.max_files
    }

    pub fn format(&self) -> LogFormat {
        self.format.clone()
    }

    pub fn is_colored(&self) -> bool {
        self.colored == Colored::On
    }

    pub fn log_dir(&self) -> Option<PathBuf> {
        self.log_dir.clone()
    }

    pub fn crates(&self) -> Vec<String> {
        self.crates.clone()
    }

    pub fn set_log_directory(self, log_dir: PathBuf) -> LoggingConfiguration {
        LoggingConfiguration {
            log_dir: Some(log_dir),
            ..self
        }
    }

    pub fn set_crates(self, crates: &[&str]) -> LoggingConfiguration {
        LoggingConfiguration {
            crates: crates.iter().map(|c| c.to_string()).collect(),
            ..self
        }
    }

    pub fn env_filter(&self) -> EnvFilter {
        let builder = EnvFilter::builder();
        builder
            .with_default_directive(self.level().into())
            .parse_lossy(
                self.crates()
                    .iter()
                    .map(|c| format!("{c}={}", self.level()))
                    .collect::<Vec<_>>()
                    .join(","),
            )
    }

    pub fn off() -> LoggingConfiguration {
        LoggingConfiguration::new(
            LevelFilter::TRACE,
            LoggingEnabled::Off,
            0,
            0,
            LogFormat::Default,
            Colored::Off,
            None,
            &[],
        )
    }

    pub fn background(log_dir: Option<PathBuf>, crates: &[&str]) -> LoggingConfiguration {
        LoggingConfiguration::new(
            log_level(LevelFilter::TRACE),
            LoggingEnabled::On,
            log_max_size_bytes(),
            log_max_files(),
            log_format(),
            Colored::Off,
            log_dir,
            crates,
        )
    }

    fn default_crates() -> Vec<String> {
        vec![
            "ockam".to_string(),
            "ockam_node".to_string(),
            "ockam_core".to_string(),
            "ockam_vault".to_string(),
            "ockam_identity".to_string(),
            "ockam_transport_tcp".to_string(),
            "ockam_api".to_string(),
            "ockam_command".to_string(),
        ]
    }
}

impl Default for LoggingConfiguration {
    fn default() -> Self {
        LoggingConfiguration::new(
            LevelFilter::TRACE,
            LoggingEnabled::On,
            100,
            60,
            LogFormat::Default,
            Colored::Off,
            None,
            &LoggingConfiguration::default_crates()
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<&str>>(),
        )
    }
}

impl Display for LoggingConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingConfiguration")
            .field("level", &self.level().to_string())
            .field("enabled", &self.enabled.to_string())
            .field("max_size_bytes", &self.max_size_bytes)
            .field("max_files", &self.max_files)
            .field("format", &self.format)
            .field("colored", &self.colored)
            .field("log_dir", &self.log_dir)
            .field("crates", &self.crates)
            .finish()
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Colored {
    On,
    Off,
}

pub fn logging_configuration(
    default_log_level: LevelFilter,
    colored: Colored,
    log_dir: Option<PathBuf>,
    crates: &[&str],
) -> LoggingConfiguration {
    legacy_logging_configuration(default_log_level, colored, log_dir.clone(), crates)
        .unwrap_or_else(|| new_logging_configuration(default_log_level, colored, log_dir, crates))
}

pub fn new_logging_configuration(
    default_log_level: LevelFilter,
    colored: Colored,
    log_dir: Option<PathBuf>,
    crates: &[&str],
) -> LoggingConfiguration {
    LoggingConfiguration::new(
        log_level(default_log_level),
        logging_enabled(),
        log_max_size_bytes(),
        log_max_files(),
        log_format(),
        colored,
        log_dir,
        crates,
    )
}

fn log_level(default_log_level: LevelFilter) -> LevelFilter {
    let log_level = match get_env::<String>("OCKAM_LOG_LEVEL").unwrap_or(None) {
        Some(s) => LevelFilter::from_str(&s).unwrap_or(default_log_level),
        None => default_log_level,
    };

    // If we end-up with a log level that is not defined but tracing is on
    // then we still need to provide a LevelFilter, since since parameter is used for both logs and traces
    if tracing_enabled() == TracingEnabled::On && log_level == LevelFilter::OFF {
        LevelFilter::TRACE
    } else {
        log_level
    }
}

fn log_max_size_bytes() -> u64 {
    let default = 100;
    get_env_with_default("OCKAM_LOG_MAX_SIZE_MB", default).unwrap_or(default) * 1024 * 1024
}

fn log_max_files() -> usize {
    let default: u64 = 60;
    get_env_with_default("OCKAM_LOG_MAX_FILES", default).unwrap_or(default) as usize
}

fn log_format() -> LogFormat {
    let default = LogFormat::Default;
    get_env_with_default("OCKAM_LOG_FORMAT", default.clone()).unwrap_or(default)
}

fn legacy_logging_configuration(
    default_log_level: LevelFilter,
    colored: Colored,
    log_dir: Option<PathBuf>,
    crates: &[&str],
) -> Option<LoggingConfiguration> {
    get_env::<String>("OCKAM_LOG")
        .unwrap_or(None)
        .map(|s| LoggingConfiguration {
            level: LevelFilter::from_str(&s).unwrap_or(default_log_level),
            enabled: LoggingEnabled::On,
            max_size_bytes: log_max_size_bytes(),
            max_files: log_max_files(),
            format: log_format(),
            colored,
            log_dir,
            crates: crates.iter().map(|c| c.to_string()).collect(),
        })
}

pub fn logging_enabled() -> LoggingEnabled {
    get_env::<LoggingEnabled>("OCKAM_LOGGING")
        .unwrap_or(None)
        .unwrap_or(LoggingEnabled::Off)
}
