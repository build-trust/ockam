use crate::config::LevelVar;
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use ockam_core::env::{get_env, get_env_with_default, is_set, FromString};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use tracing_core::Level;
use tracing_subscriber::EnvFilter;

use super::{Colored, GlobalErrorHandler, LoggingEnabled};
use crate::logs::LogFormat;

/// List of all the configuration parameters relevant for configuring the logs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfiguration {
    /// Specifies if logging is enabled
    enabled: LoggingEnabled,
    /// Verbosity required for a given span or log record
    level: Level,
    /// This parameter specifies what to do when there are logging or tracing errors
    global_error_handler: GlobalErrorHandler,
    /// Maximum log file size in bytes
    max_size_bytes: u64,
    /// Maximum number of log files for a given node
    max_files: u64,
    /// Format used for log lines: pretty, json, default
    format: LogFormat,
    /// This parameter specifies if the log output is colored (typically in terminals supporting it)
    colored: Colored,
    /// Director where log files must be created.
    /// If no directory is defined then log messages appear on the console
    log_dir: Option<PathBuf>,
    /// List of create for which we want to keep log messages
    crates: Option<Vec<String>>,
}

impl LoggingConfiguration {
    /// Create a new logging configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        enabled: LoggingEnabled,
        level: Level,
        global_error_handler: GlobalErrorHandler,
        max_size_bytes: u64,
        max_files: u64,
        format: LogFormat,
        colored: Colored,
        log_dir: Option<PathBuf>,
        crates_filter: CratesFilter,
    ) -> LoggingConfiguration {
        let crates = match crates_filter {
            CratesFilter::All => None,
            CratesFilter::Default => Some(LoggingConfiguration::default_crates()),
            CratesFilter::Selected(crates) => Some(crates),
        };

        LoggingConfiguration {
            enabled,
            level,
            global_error_handler,
            max_size_bytes,
            max_files,
            format,
            colored,
            log_dir,
            crates,
        }
    }

    /// Return true if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled == LoggingEnabled::On
    }

    /// Return the logging level
    pub fn level(&self) -> Level {
        self.level
    }

    /// Return the desired global error handler
    pub fn global_error_handler(&self) -> GlobalErrorHandler {
        self.global_error_handler
    }

    /// Return the maximum log file size
    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_size_bytes
    }

    /// Return the maximum number of log files for a given node
    pub fn max_files(&self) -> u64 {
        self.max_files
    }

    /// Return the log format used for log lines
    pub fn format(&self) -> LogFormat {
        self.format.clone()
    }

    /// Return true if color can be used for log lines
    pub fn is_colored(&self) -> bool {
        self.colored == Colored::On
    }

    /// Return the (optional) directory for creating log files
    pub fn log_dir(&self) -> Option<PathBuf> {
        self.log_dir.clone()
    }

    /// Return the crates used to filter log messages
    pub fn crates(&self) -> Option<Vec<String>> {
        self.crates.clone()
    }

    /// Set a specific log directory
    pub fn set_log_directory(self, log_dir: PathBuf) -> LoggingConfiguration {
        LoggingConfiguration {
            log_dir: Some(log_dir),
            ..self
        }
    }

    /// Set some specific crates
    pub fn set_crates(self, crates: &[&str]) -> LoggingConfiguration {
        LoggingConfiguration {
            crates: Some(crates.iter().map(|c| c.to_string()).collect()),
            ..self
        }
    }

    /// Set the default crates
    pub fn set_default_crates(self) -> LoggingConfiguration {
        LoggingConfiguration {
            crates: Some(LoggingConfiguration::default_crates()),
            ..self
        }
    }

    /// Set the all crates
    pub fn set_all_crates(self) -> LoggingConfiguration {
        LoggingConfiguration {
            crates: None,
            ..self
        }
    }

    /// Set the log level crates
    pub fn set_log_level(self, level: Level) -> LoggingConfiguration {
        LoggingConfiguration { level, ..self }
    }

    /// Create an EnvFilter which keeps only the log messages
    ///
    ///  - for the configured level
    ///  - for the configured crates
    pub fn env_filter(&self) -> EnvFilter {
        match &self.crates {
            Some(crates) => {
                let builder = EnvFilter::builder();
                builder
                    .with_default_directive(self.level().into())
                    .parse_lossy(
                        crates
                            .iter()
                            .map(|c| format!("{c}={}", self.level()))
                            .collect::<Vec<_>>()
                            .join(","),
                    )
            }
            None => EnvFilter::default().add_directive(self.level.into()),
        }
    }

    /// Return a LoggingConfiguration which doesn't log anything
    /// We still retrieve:
    ///  - the log level
    ///  - the global error handler
    ///  - the crates filter
    ///
    /// Since those pieces of configuration are used by both logging and tracing
    pub fn off() -> ockam_core::Result<LoggingConfiguration> {
        Ok(LoggingConfiguration::new(
            LoggingEnabled::Off,
            log_level(None)?,
            global_error_handler()?,
            0,
            0,
            LogFormat::Default,
            Colored::Off,
            None,
            crates_filter()?,
        ))
    }

    /// Return a LoggingConfiguration which creates logs for a background node
    pub fn background(
        log_dir: Option<PathBuf>,
        crates_filter: CratesFilter,
    ) -> ockam_core::Result<LoggingConfiguration> {
        Ok(LoggingConfiguration::new(
            LoggingEnabled::On,
            log_level(None)?,
            global_error_handler()?,
            log_max_size_bytes()?,
            log_max_files()?,
            log_format()?,
            Colored::Off,
            log_dir,
            crates_filter,
        ))
    }

    /// List of default crates to keep for log messages
    pub fn default_crates() -> Vec<String> {
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

impl Display for LoggingConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingConfiguration")
            .field("enabled", &self.enabled.to_string())
            .field("level", &self.level().to_string())
            .field(
                "global_error_handler",
                &self.global_error_handler.to_string(),
            )
            .field("max_size_bytes", &self.max_size_bytes)
            .field("max_files", &self.max_files)
            .field("format", &self.format)
            .field("colored", &self.colored)
            .field("log_dir", &self.log_dir)
            .field("crates", &self.crates)
            .finish()
    }
}

/// Create the logging configuration
/// If the OCKAM_LOG variable is specified we take the preferred log level from that variable.
/// Otherwise we take it from the OCKAM_LOG_LEVEL variable
pub fn logging_configuration(
    preferred_log_level: Option<Level>,
    colored: Colored,
    log_dir: Option<PathBuf>,
    crates: CratesFilter,
) -> ockam_core::Result<LoggingConfiguration> {
    let is_legacy_variable_set = is_set::<LevelVar>(OCKAM_LOG)?;
    let enabled = if is_legacy_variable_set || preferred_log_level.is_some() {
        LoggingEnabled::On
    } else {
        logging_enabled()?
    };
    let level = log_level(preferred_log_level)?;
    Ok(LoggingConfiguration::new(
        enabled,
        level,
        global_error_handler()?,
        log_max_size_bytes()?,
        log_max_files()?,
        log_format()?,
        colored,
        log_dir,
        crates,
    ))
}

/// Return the maximum size of a log file, taken from an environment variable
fn log_max_size_bytes() -> ockam_core::Result<u64> {
    get_env_with_default(OCKAM_LOG_MAX_SIZE_MB, DEFAULT_LOG_MAX_SIZE_MB).map(|v| v * 1024 * 1024)
}

/// Return the maximum number of log files per node, taken from an environment variable
fn log_max_files() -> ockam_core::Result<u64> {
    get_env_with_default(OCKAM_LOG_MAX_FILES, DEFAULT_LOG_MAX_FILES)
}

/// Return the format to use for log messages, taken from an environment variable
fn log_format() -> ockam_core::Result<LogFormat> {
    get_env_with_default(OCKAM_LOG_FORMAT, DEFAULT_LOG_FORMAT)
}

/// Return a value setting the logging on or off, taken from an environment variable
pub fn logging_enabled() -> ockam_core::Result<LoggingEnabled> {
    match get_env::<bool>(OCKAM_LOGGING)? {
        Some(v) => Ok(if v {
            LoggingEnabled::On
        } else {
            LoggingEnabled::Off
        }),
        None => Ok(LoggingEnabled::Off),
    }
}

/// Return the log level, using the legacy environment variable if defined,
/// or the current one.
fn log_level(preferred_log_level: Option<Level>) -> ockam_core::Result<Level> {
    let is_legacy_variable_set = is_set::<LevelVar>(OCKAM_LOG)?;
    let env_variable = if is_legacy_variable_set {
        println!("The OCKAM_LOG variable is deprecated. Please use: OCKAM_LOGGING=true OCKAM_LOG_LEVEL=trace (with the desired level) instead.");
        OCKAM_LOG
    } else {
        OCKAM_LOG_LEVEL
    };
    get_log_level(env_variable, preferred_log_level)
}

/// A user can pass a preferred logging level via a -vvv argument (with the number of 'v' indicating
/// the log level). If that argument is not present, we get the log level from an environment variable.
fn get_log_level(
    variable_name: &str,
    preferred_log_level: Option<Level>,
) -> ockam_core::Result<Level> {
    match preferred_log_level {
        Some(level) => Ok(level),
        None => get_env_with_default(
            variable_name,
            LevelVar {
                level: Level::DEBUG,
            },
        )
        .map(|l| l.level),
    }
}

/// Return the strategy to use for reporting logging/tracing errors
pub fn global_error_handler() -> ockam_core::Result<GlobalErrorHandler> {
    match get_env::<GlobalErrorHandler>(OCKAM_TRACING_GLOBAL_ERROR_HANDLER)? {
        Some(v) => Ok(v),
        None => Ok(GlobalErrorHandler::LogFile),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CratesFilter {
    All,
    Default,
    Selected(Vec<String>),
}

impl FromString for CratesFilter {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "all" => Ok(CratesFilter::All),
            "default" => Ok(CratesFilter::Default),
            other => Ok(CratesFilter::Selected(<Vec<String>>::from_string(other)?)),
        }
    }
}

impl Display for CratesFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CratesFilter::All => f.write_str("all"),
            CratesFilter::Default => f.write_str("default"),
            CratesFilter::Selected(s) => f.write_str(s.join(",").as_str()),
        }
    }
}

/// Return the crates which should generate logs. If the variable is not defined,
/// then the ockam crates are returned.
pub fn crates_filter() -> ockam_core::Result<CratesFilter> {
    get_env_with_default(OCKAM_LOG_CRATES_FILTER, CratesFilter::Default)
}
