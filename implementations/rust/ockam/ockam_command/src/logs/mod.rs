use crate::logs::rolling::{RollingConditionBasic, RollingFileAppender};

use ockam_core::env::{get_env, get_env_with_default, FromString};
use std::io::stdout;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[allow(unused, clippy::enum_variant_names)]
mod rolling;

fn log_max_size() -> u64 {
    let default = 100;
    get_env_with_default("OCKAM_LOG_MAX_SIZE_MB", default).unwrap_or(default)
}

fn log_max_files() -> usize {
    let default: u64 = 60;
    get_env_with_default("OCKAM_LOG_MAX_FILES", default).unwrap_or(default) as usize
}

fn log_format() -> LogFormat {
    let default = LogFormat::Default;
    get_env_with_default("OCKAM_LOG_FORMAT", default.clone()).unwrap_or(default)
}

#[derive(Clone)]
enum LogFormat {
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

pub fn setup_logging(
    verbose: u8,
    no_color: bool,
    is_tty: bool,
    log_path: Option<PathBuf>,
) -> Option<WorkerGuard> {
    let level = {
        // Parse the the raw log level value (e.g. "info" or "-vvv").
        let level_raw = match get_env::<String>("OCKAM_LOG") {
            // If OCKAM_LOG is set, give it priority over `verbose` to define the log level.
            Ok(Some(s)) if !s.is_empty() => s,
            // Otherwise, use `verbose` to define the log level.
            Ok(_) | Err(_) => match verbose {
                0 => "off".to_string(),
                1 => "info".to_string(),
                2 => "debug".to_string(),
                _ => "trace".to_string(),
            },
        };
        // If the parsed log level is not valid, default to info.
        let level = LevelFilter::from_str(&level_raw).unwrap_or(LevelFilter::INFO);
        if level == LevelFilter::OFF {
            return None;
        }
        level
    };
    let filter = {
        let ockam_crates = [
            "ockam",
            "ockam_node",
            "ockam_core",
            "ockam_vault",
            "ockam_identity",
            "ockam_transport_tcp",
            "ockam_api",
            "ockam_command",
        ];
        let builder = EnvFilter::builder();
        builder
            .with_default_directive(level.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}={level}")).join(","))
    };
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_error::ErrorLayer::default());
    let (appender, guard) = match log_path {
        // If a log path is not provided, log to stdout.
        None => {
            let color = !no_color && is_tty;
            let (n, guard) = tracing_appender::non_blocking(stdout());
            let appender = layer().with_ansi(color).with_writer(n);
            (Box::new(appender), guard)
        }
        // If a log path is provided, log to a rolling file appender.
        Some(log_path) => {
            let r = RollingFileAppender::new(
                log_path,
                RollingConditionBasic::new()
                    .daily()
                    .max_size(log_max_size() * 1024 * 1024),
                log_max_files(),
            )
            .expect("Failed to create rolling file appender");
            let (n, guard) = tracing_appender::non_blocking(r);
            let appender = layer().with_ansi(false).with_writer(n);
            (Box::new(appender), guard)
        }
    };
    let res = match log_format() {
        LogFormat::Pretty => subscriber.with(appender.pretty()).try_init(),
        LogFormat::Json => subscriber.with(appender.json()).try_init(),
        LogFormat::Default => subscriber.with(appender).try_init(),
    };
    res.expect("Failed to initialize tracing subscriber");
    Some(guard)
}
