use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::{
    logging_configuration, Colored, ExportingConfiguration, LogLevelWithCratesFilter,
    LoggingTracing,
};

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
        let level_and_crates = LogLevelWithCratesFilter::from_verbose(2)
            .unwrap()
            .add_crates(vec!["ockam_app_lib"]);
        let tracing_guard = LoggingTracing::setup(
            &logging_configuration(level_and_crates, Some(node_dir), Colored::Off).unwrap(),
            &ExportingConfiguration::foreground().unwrap(),
            "portals",
            Some("portals".to_string()),
        );
        self.tracing_guard
            .set(tracing_guard)
            .expect("Failed to initialize logs");
    }
}
