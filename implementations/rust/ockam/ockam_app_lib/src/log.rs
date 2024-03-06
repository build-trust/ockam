use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::{
    logging_configuration, Colored, CratesFilter, ExportingConfiguration, LoggingTracing,
};
use tracing_core::Level;

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
        let ockam_crates = CratesFilter::Selected(vec![
            "ockam".to_string(),
            "ockam_node".to_string(),
            "ockam_core".to_string(),
            "ockam_vault".to_string(),
            "ockam_identity".to_string(),
            "ockam_transport_tcp".to_string(),
            "ockam_api".to_string(),
            "ockam_command".to_string(),
            "ockam_app_lib".to_string(),
        ]);

        let tracing_guard = LoggingTracing::setup(
            &logging_configuration(
                Some(Level::DEBUG),
                Colored::Off,
                Some(node_dir),
                ockam_crates,
            )
            .unwrap(),
            &ExportingConfiguration::foreground(true).unwrap(),
            "portals",
            Some("portals".to_string()),
        );
        self.tracing_guard
            .set(tracing_guard)
            .expect("Failed to initialize logs");
    }
}
