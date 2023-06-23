use crate::logs::rolling::{RollingConditionBasic, RollingFileAppender};

use ockam_core::env::{get_env, get_env_with_default};
use std::io::stdout;
use std::path::PathBuf;
use std::str::FromStr;
use termimad::crossterm::tty::IsTty;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

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

pub fn setup_logging(
    verbose: u8,
    no_color: bool,
    log_path: Option<PathBuf>,
) -> Option<WorkerGuard> {
    let ockam_crates = [
        "ockam",
        "ockam_node",
        "ockam_core",
        "ockam_command",
        "ockam_identity",
        "ockam_transport_tcp",
        "ockam_vault",
    ];
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
    let builder = EnvFilter::builder();
    let filter = builder
        .with_default_directive(level.into())
        .parse_lossy(ockam_crates.map(|c| format!("{c}={level}")).join(","));
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_error::ErrorLayer::default());
    let (subscriber, guard) = match log_path {
        // If a log path is not provided, log to stdout.
        None => {
            let color = !no_color && stdout().is_tty();
            let (n, guard) = tracing_appender::non_blocking(stdout());
            let fmt = tracing_subscriber::fmt::Layer::default()
                .with_ansi(color)
                .with_writer(n);
            (subscriber.with(fmt).try_init(), Some(guard))
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
            let fmt = tracing_subscriber::fmt::Layer::default()
                .with_ansi(!no_color)
                .with_writer(n);
            (subscriber.with(fmt).try_init(), Some(guard))
        }
    };
    subscriber.expect("Failed to initialize tracing subscriber");
    guard
}
