use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::{logging_configuration, tracing_configuration, Colored, LoggingTracing};
use tracing_core::LevelFilter;

impl AppState {
    /// Setup logging and tracing for the Portals application
    /// If this has not been done yet
    pub fn setup_logging_tracing(&self) {
        if self.tracing_guard.get().is_some() {
            return;
        }
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
        let guard = LoggingTracing::setup(
            logging_configuration(
                None,
                LevelFilter::TRACE,
                Colored::Off,
                Some(node_dir),
                &ockam_crates,
            ),
            tracing_configuration(),
            "portals",
        );
        self.tracing_guard
            .set(guard)
            .expect("Failed to initialize logs");
    }
}
