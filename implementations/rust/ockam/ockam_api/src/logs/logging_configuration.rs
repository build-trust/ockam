use crate::config::LevelVar;
use crate::logs::default_values::*;
use crate::logs::env_variables::*;
use ockam_core::env::{get_env, get_env_with_default, FromString};
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
        LoggingConfiguration {
            enabled,
            level,
            global_error_handler,
            max_size_bytes,
            max_files,
            format,
            colored,
            log_dir,
            crates: crates_filter.crates(),
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
        let level_and_crates = LogLevelWithCratesFilter::new()?;
        Ok(LoggingConfiguration::new(
            LoggingEnabled::Off,
            level_and_crates.level,
            global_error_handler()?,
            0,
            0,
            LogFormat::Default,
            Colored::Off,
            None,
            level_and_crates.crates_filter,
        ))
    }

    /// Return a LoggingConfiguration which creates logs for a background node
    pub fn background(log_dir: Option<PathBuf>) -> ockam_core::Result<LoggingConfiguration> {
        let level_and_crates = LogLevelWithCratesFilter::new()?;
        Ok(LoggingConfiguration::new(
            LoggingEnabled::On,
            level_and_crates.level,
            global_error_handler()?,
            log_max_size_bytes()?,
            log_max_files()?,
            log_format()?,
            Colored::Off,
            log_dir,
            level_and_crates.crates_filter.clone(),
        ))
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

/// Create the logging configuration.
/// If the verbose flag is set, we enable logging.
/// Otherwise, we use the OCKAM_LOGGING variable to decide.
pub fn logging_configuration(
    level_and_crates: LogLevelWithCratesFilter,
    log_dir: Option<PathBuf>,
    colored: Colored,
) -> ockam_core::Result<LoggingConfiguration> {
    let enabled = if level_and_crates.explicit_verbose_flag {
        LoggingEnabled::On
    } else {
        logging_enabled()?
    };
    Ok(LoggingConfiguration::new(
        enabled,
        level_and_crates.level,
        global_error_handler()?,
        log_max_size_bytes()?,
        log_max_files()?,
        log_format()?,
        colored,
        log_dir,
        level_and_crates.crates_filter.clone(),
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

/// Return the strategy to use for reporting logging/tracing errors
pub fn global_error_handler() -> ockam_core::Result<GlobalErrorHandler> {
    match get_env::<GlobalErrorHandler>(OCKAM_TRACING_GLOBAL_ERROR_HANDLER)? {
        Some(v) => Ok(v),
        None => Ok(GlobalErrorHandler::LogFile),
    }
}

/// Represents the relationship between a log level and the crates that will
/// get monitored at that level
pub struct LogLevelWithCratesFilter {
    pub level: Level,
    pub crates_filter: CratesFilter,
    /// Indicates if the log level was explicitly set by the commands' `verbose` flag
    pub explicit_verbose_flag: bool,
}

impl LogLevelWithCratesFilter {
    /// Create a new LogLevel based on the environment variables for the log level and crates filter
    pub fn new() -> ockam_core::Result<Self> {
        let level = Self::get_log_level_from_env()?;
        let crates_filter = match CratesFilter::try_from_env()? {
            None => match level {
                Level::INFO => CratesFilter::Basic,
                _ => CratesFilter::Core,
            },
            Some(f) => f,
        };
        Ok(Self {
            level,
            crates_filter,
            explicit_verbose_flag: false,
        })
    }

    /// Create a new LogLevel, using the verbose argument if > 0, or one of the environment variables.
    pub fn from_verbose(verbose: u8) -> ockam_core::Result<Self> {
        let level = match verbose {
            0 => Self::get_log_level_from_env()?,
            1 | 2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        };
        let crates_filter = CratesFilter::from_verbose(verbose)?;
        let explicit_verbose_flag = verbose > 0;
        Ok(Self {
            level,
            crates_filter,
            explicit_verbose_flag,
        })
    }

    /// Return the log level based on the log level environment variable.
    /// Default to DEBUG.
    fn get_log_level_from_env() -> ockam_core::Result<Level> {
        get_env_with_default(
            OCKAM_LOG_LEVEL,
            LevelVar {
                level: Level::DEBUG,
            },
        )
        .map(|l| l.level)
    }

    pub fn add_crates(self, new: Vec<impl Into<String>>) -> Self {
        let crates = self
            .crates_filter
            .crates()
            .unwrap_or_default()
            .into_iter()
            .chain(new.into_iter().map(Into::into))
            .collect();
        Self {
            crates_filter: CratesFilter::Selected(crates),
            ..self
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CratesFilter {
    All,
    Basic,
    Core,
    Selected(Vec<String>),
}

impl CratesFilter {
    pub fn try_from_env() -> ockam_core::Result<Option<Self>> {
        get_env_with_default(OCKAM_LOG_CRATES_FILTER, None)
    }

    /// Use the verbose flag if set, otherwise use the environment variable or default to `Basic`
    pub fn from_verbose(verbose: u8) -> ockam_core::Result<Self> {
        Ok(match verbose {
            0 => get_env_with_default(OCKAM_LOG_CRATES_FILTER, CratesFilter::Basic)?,
            1 => CratesFilter::Basic,
            2..=4 => CratesFilter::Core,
            _ => CratesFilter::All,
        })
    }

    /// List of crates to keep for log messages
    pub fn crates(&self) -> Option<Vec<String>> {
        match self {
            CratesFilter::All => None,
            CratesFilter::Basic => Some(vec![
                "ockam_api::ui::terminal".to_string(),
                "ockam_command".to_string(),
            ]),
            CratesFilter::Core => Some(vec![
                "ockam".to_string(),
                "ockam_node".to_string(),
                "ockam_core".to_string(),
                "ockam_vault".to_string(),
                "ockam_identity".to_string(),
                "ockam_transport_tcp".to_string(),
                "ockam_api".to_string(),
                "ockam_command".to_string(),
            ]),
            CratesFilter::Selected(list) => Some(list.clone()),
        }
    }
}

impl FromString for CratesFilter {
    fn from_string(s: &str) -> ockam_core::Result<Self> {
        match s {
            "all" => Ok(CratesFilter::All),
            "basic" => Ok(CratesFilter::Basic),
            "core" => Ok(CratesFilter::Core),
            other => Ok(CratesFilter::Selected(<Vec<String>>::from_string(other)?)),
        }
    }
}

impl Display for CratesFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CratesFilter::All => f.write_str("all"),
            CratesFilter::Basic => f.write_str("basic"),
            CratesFilter::Core => f.write_str("core"),
            CratesFilter::Selected(s) => f.write_str(s.join(",").as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn log_level_with_crates_filter_new() {
        std::env::remove_var(OCKAM_LOG_LEVEL);
        std::env::remove_var(OCKAM_LOG_CRATES_FILTER);

        // No env vars defined. Use default values.
        let sut = LogLevelWithCratesFilter::new().unwrap();
        assert_eq!(sut.level, Level::DEBUG);
        assert_eq!(sut.crates_filter, CratesFilter::Core);
        assert!(!sut.explicit_verbose_flag);

        // Override with env vars
        std::env::set_var(OCKAM_LOG_LEVEL, "info");
        std::env::set_var(OCKAM_LOG_CRATES_FILTER, "my_crate,other_crate");
        let sut = LogLevelWithCratesFilter::new().unwrap();
        assert_eq!(sut.level, Level::INFO);
        assert_eq!(
            sut.crates_filter,
            CratesFilter::Selected(vec!["my_crate".to_string(), "other_crate".to_string()])
        );
        assert!(!sut.explicit_verbose_flag);
    }

    #[test]
    #[serial]
    fn log_level_with_crates_filter_with_verbose() {
        std::env::remove_var(OCKAM_LOG_LEVEL);
        std::env::remove_var(OCKAM_LOG_CRATES_FILTER);

        // No env vars defined. Use verbose value.
        let sut = LogLevelWithCratesFilter::from_verbose(1).unwrap();
        assert_eq!(sut.level, Level::INFO);
        assert_eq!(sut.crates_filter, CratesFilter::Basic);
        assert!(sut.explicit_verbose_flag);

        // Set env vars
        std::env::set_var(OCKAM_LOG_LEVEL, "trace");
        std::env::set_var(OCKAM_LOG_CRATES_FILTER, "my_crate");

        // If verbose is 0, fallback to env vars
        let sut = LogLevelWithCratesFilter::from_verbose(0).unwrap();
        assert_eq!(sut.level, Level::TRACE);
        assert_eq!(
            sut.crates_filter,
            CratesFilter::Selected(vec!["my_crate".to_string()])
        );
        assert!(!sut.explicit_verbose_flag);

        // If verbose is > 0, ignore env vars
        let sut = LogLevelWithCratesFilter::from_verbose(1).unwrap();
        assert_eq!(sut.level, Level::INFO);
        assert_eq!(sut.crates_filter, CratesFilter::Basic);
        assert!(sut.explicit_verbose_flag);
    }
}
