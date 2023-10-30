use crate::state::{AppState, NODE_NAME};
use ockam_core::env::get_env;
use std::fs::OpenOptions;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;

impl AppState {
    pub fn setup_logging(&self) {
        let level = if let Some(raw) = get_env::<String>("OCKAM_LOG").unwrap_or_default() {
            LevelFilter::from_str(&raw).unwrap_or(LevelFilter::INFO)
        } else {
            LevelFilter::INFO
        };
        let file = {
            let this = self.clone();
            let state = self
                .context()
                .runtime()
                .block_on(async move { this.state().await });
            let log_path = state
                .nodes
                .stdout_logs(NODE_NAME)
                .expect("Failed to get stdout log path for node");
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .expect("Failed to open stdout log file")
        };
        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_ansi(false)
            .with_writer(file)
            .init();
    }
}
