use ockam_api::logs::env::{
    log_level, logging_enabled, tracing_enabled, LoggingEnabled, TracingEnabled,
};
use ockam_api::logs::{LevelFilter, Logging, TracingGuard};
use std::path::PathBuf;
use std::str::FromStr;

pub fn setup_logging(
    background_node: bool,
    verbose: u8,
    no_color: bool,
    is_tty: bool,
    log_path: Option<PathBuf>,
) -> TracingGuard {
    let level = {
        // Parse the the raw log level value (e.g. "info" or "-vvv").
        let level_raw = match log_level() {
            // If OCKAM_LOG_LEVEL is set, give it priority over `verbose` to define the log level.
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
        LevelFilter::from_str(&level_raw).unwrap_or(LevelFilter::INFO)
    };
    let color = !no_color && is_tty;
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
    if background_node {
        Logging::setup(
            LevelFilter::TRACE,
            LoggingEnabled::On,
            TracingEnabled::On,
            color,
            log_path,
            &ockam_crates,
        )
    } else {
        Logging::setup(
            level,
            logging_enabled(),
            tracing_enabled(),
            color,
            log_path,
            &ockam_crates,
        )
    }
}
