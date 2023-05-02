use crate::logs::rolling::{RollingConditionBasic, RollingFileAppender};

use ockam_core::env::{get_env, get_env_with_default};
use std::io::stdout;
use std::path::PathBuf;
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
    let builder = EnvFilter::builder();

    // Otherwise, use `verbose` to define the log level.
    let filter = match (stdout().is_tty(), verbose) {
        // If tty and `verbose` is not set, try to read the log level from the OCKAM_LOG env variable.
        (true, 0) => match get_env::<String>("OCKAM_LOG") {
            Ok(Some(s)) if !s.is_empty() => builder.with_env_var("OCKAM_LOG").from_env_lossy(),
            // Default to info if OCKAM_LOG is not set.
            _ => builder
                .with_default_directive(LevelFilter::INFO.into())
                .parse_lossy(ockam_crates.map(|c| format!("{c}=info")).join(",")),
        },
        // If not tty, default to `info` level.
        (false, 0) | (_, 1) => builder
            .with_default_directive(LevelFilter::INFO.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=info")).join(",")),
        // In all other cases, default to the level set by `verbose`.
        (_, 2) => builder
            .with_default_directive(LevelFilter::DEBUG.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=debug")).join(",")),
        _ => builder
            .with_default_directive(LevelFilter::TRACE.into())
            .parse_lossy(ockam_crates.map(|c| format!("{c}=trace")).join(",")),
    };
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_error::ErrorLayer::default());
    let (subscriber, guard) = match (verbose, log_path) {
        (0, None) => return None,
        (_, None) => {
            let color = !no_color && stdout().is_tty();
            let (n, guard) = tracing_appender::non_blocking(stdout());
            let fmt = tracing_subscriber::fmt::Layer::default()
                .with_ansi(color)
                .with_writer(n);
            (subscriber.with(fmt).try_init(), Some(guard))
        }
        (_, Some(log_path)) => {
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
