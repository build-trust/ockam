use crate::logs::env::{log_format, log_max_files};
use ockam_core::env::FromString;
use std::io::stdout;
use std::path::PathBuf;
pub use tracing::level_filters::LevelFilter;
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod env;

pub struct Logging;

impl Logging {
    pub fn setup(
        level: LevelFilter,
        color: bool,
        node_dir: Option<PathBuf>,
        crates: &[&str],
    ) -> Option<WorkerGuard> {
        let filter = {
            let builder = EnvFilter::builder();
            builder.with_default_directive(level.into()).parse_lossy(
                crates
                    .iter()
                    .map(|c| format!("{c}={level}"))
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(tracing_error::ErrorLayer::default());
        let (appender, guard) = match node_dir {
            // If a node dir path is not provided, log to stdout.
            None => {
                let (n, guard) = tracing_appender::non_blocking(stdout());
                let appender = layer().with_ansi(color).with_writer(n);
                (Box::new(appender), guard)
            }
            // If a log path is provided, log to a rolling file appender.
            Some(node_dir) => {
                let r = RollingFileAppender::builder()
                    .rotation(Rotation::DAILY)
                    .max_log_files(log_max_files())
                    .filename_prefix("stdout")
                    .filename_suffix("log")
                    .build(node_dir)
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
}

#[derive(Clone)]
pub enum LogFormat {
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
