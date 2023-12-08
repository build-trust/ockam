use ockam_api::logs::env::log_level;
use ockam_api::logs::{LevelFilter, Logging, WorkerGuard};
use std::path::PathBuf;
use std::str::FromStr;

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
    Logging::setup(level, color, log_path, &ockam_crates)
}
