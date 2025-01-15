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

        self.context()
            .runtime()
            .block_on(async move { self.setup_logging_tracing_impl().await });
    }

    async fn setup_logging_tracing_impl(&self) {
        let state = self.state().await;
        let node_dir = state
            .node_dir(NODE_NAME)
            .expect("Failed to get node directory");
        let level_and_crates = LogLevelWithCratesFilter::from_verbose(2)
            .unwrap()
            .add_crates(vec!["ockam_app_lib"]);
        let tracing_guard = LoggingTracing::setup(
            &logging_configuration(level_and_crates, Some(node_dir), Colored::Off).unwrap(),
            &ExportingConfiguration::foreground(&state).await.unwrap(),
            "portals",
            Some("portals".to_string()),
        );
        self.tracing_guard
            .set(tracing_guard)
            .expect("Failed to initialize logs");
    }
}
