use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::env::log_level;
use ockam_api::logs::{LevelFilter, Logging};
use std::str::FromStr;

impl AppState {
    pub fn setup_logging(&self) {
        if self.tracing_guard.get().is_some() {
            return;
        }
        let level = log_level()
            .map(|s| LevelFilter::from_str(&s))
            .and_then(Result::ok)
            .unwrap_or(LevelFilter::INFO);
        let node_dir = {
            let this = self.clone();
            let state = self
                .context()
                .runtime()
                .block_on(async move { this.state().await });
            state.node_dir(NODE_NAME)
        };
        let ockam_crates = [
            "ockam",
            "ockam_node",
            "ockam_core",
            "ockam_vault",
            "ockam_identity",
            "ockam_transport_tcp",
            "ockam_api",
            "ockam_command",
            "ockam_app_lib",
        ];
        if let Some(guard) = Logging::setup(level, false, Some(node_dir), &ockam_crates) {
            self.tracing_guard
                .set(guard)
                .expect("Failed to initialize logs");
        }
    }
}
