use crate::state::{AppState, NODE_NAME};
use ockam_core::env::get_env;
use std::fs::OpenOptions;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;

impl AppState {
    pub fn setup_logging(&self) {
        let this = self.clone();
        self.context().runtime().block_on(async move {
            let level = if let Some(raw) = get_env::<String>("OCKAM_LOG").unwrap_or_default() {
                LevelFilter::from_str(&raw).unwrap_or(LevelFilter::INFO)
            } else {
                LevelFilter::INFO
            };

            let log_path = this.state().await.nodes.stdout_logs(NODE_NAME).unwrap();
            let open_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .unwrap();

            tracing_subscriber::fmt()
                .with_max_level(level)
                .with_ansi(false)
                .with_writer(open_file)
                .init();
        });
    }
}
