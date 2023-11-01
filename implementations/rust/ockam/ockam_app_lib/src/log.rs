use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::env::{log_format, log_level, log_max_files, log_max_size_bytes};
use ockam_api::logs::rolling::{RollingConditionBasic, RollingFileAppender};
use ockam_api::logs::LogFormat;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

impl AppState {
    pub fn setup_logging(&self) {
        if self.tracing_guard.get().is_some() {
            return;
        }

        let log_path = {
            let this = self.clone();
            let state = self
                .context()
                .runtime()
                .block_on(async move { this.state().await });
            state
                .nodes
                .stdout_logs(NODE_NAME)
                .expect("Failed to get stdout log path for node")
        };
        let level = log_level()
            .map(|s| LevelFilter::from_str(&s))
            .and_then(Result::ok)
            .unwrap_or(LevelFilter::INFO);
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
        let (appender, guard) = {
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
        };
        let res = match log_format() {
            LogFormat::Pretty => subscriber.with(appender.pretty()).try_init(),
            LogFormat::Json => subscriber.with(appender.json()).try_init(),
            LogFormat::Default => subscriber.with(appender).try_init(),
        };
        res.expect("Failed to initialize tracing subscriber");

        self.tracing_guard
            .set(guard)
            .expect("Failed to initialize logs");
    }
}
