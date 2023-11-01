use ockam_api::logs::env::{log_format, log_level, log_max_files, log_max_size_bytes};
use ockam_api::logs::rolling::{RollingConditionBasic, RollingFileAppender};
use ockam_api::logs::LogFormat;
use std::io::stdout;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn setup_logging(
    verbose: u8,
    no_color: bool,
    is_tty: bool,
    log_path: Option<PathBuf>,
) -> Option<WorkerGuard> {
    let level = {
        // Parse the the raw log level value (e.g. "info" or "-vvv").
        let level_raw = match log_level() {
            // If OCKAM_LOG is set, give it priority over `verbose` to define the log level.
            Some(s) if !s.is_empty() => s,
            // Otherwise, use `verbose` to define the log level.
            _ => match verbose {
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
                    .max_size(log_max_size_bytes()),
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
