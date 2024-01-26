use ockam_api::logs::{
    logging_configuration, tracing_configuration, Colored, LoggingConfiguration, LoggingTracing,
    TracingGuard,
};
use std::path::PathBuf;
use tracing_core::LevelFilter;

/// Set up a logger and a tracer for the current node
/// If the node is a background node we always enable logging, regardless of environment variables
pub fn setup_logging_tracing(
    background_node: bool,
    verbose: u8,
    no_color: bool,
    is_tty: bool,
    log_path: Option<PathBuf>,
) -> TracingGuard {
    let colored = if !no_color && is_tty {
        Colored::On
    } else {
        Colored::Off
    };
    let default_log_level = verbose_log_level(verbose);

    if background_node {
        LoggingTracing::setup(
            LoggingConfiguration::background(log_path, LoggingConfiguration::default_crates()),
            tracing_configuration(),
            "local node",
        )
    } else {
        LoggingTracing::setup(
            logging_configuration(
                default_log_level,
                colored,
                log_path,
                LoggingConfiguration::default_crates(),
            ),
            tracing_configuration(),
            "cli",
        )
    }
}

/// Return the LevelFilter corresponding to a given verbose flag
/// -vvv set verbose to 3 (3 times 'v')
/// and this function translate the number of `v` to a log level
fn verbose_log_level(verbose: u8) -> LevelFilter {
    match verbose {
        0 => LevelFilter::OFF,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}
