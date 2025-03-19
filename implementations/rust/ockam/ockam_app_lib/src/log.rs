use crate::state::{AppState, NODE_NAME};
use ockam_api::logs::{
    logging_configuration, logging_enabled, Colored, ExportingConfiguration, LogFormat,
    LogLevelWithCratesFilter, LoggingTracing,
};
use ockam_core::TryClone;
use ockam_node::Context;

impl AppState {
    /// Setup logging and tracing for the Portals application
    /// If this has not been done yet
    pub fn setup_logging_tracing(&self) {
        if self.tracing_guard.get().is_some() {
            return;
        }

        let ctx = self.context();
        if let Ok(ctx_clone) = ctx.try_clone() {
            ctx.runtime()
                .block_on(async move { self.setup_logging_tracing_impl(&ctx_clone).await });
        }
    }

    async fn setup_logging_tracing_impl(&self, ctx: &Context) {
        let state = self.state().await;
        let node_dir = state
            .node_dir(NODE_NAME)
            .expect("Failed to get node directory");
        let level_and_crates = LogLevelWithCratesFilter::from_verbose(2)
            .unwrap()
            .add_crates(vec!["ockam_app_lib"]);
        let tracing_guard = LoggingTracing::setup(
            state.clone(),
            &logging_configuration(
                level_and_crates,
                Some(node_dir),
                Colored::Off,
                LogFormat::Default,
                logging_enabled().unwrap(),
            )
            .unwrap(),
            &ExportingConfiguration::foreground(&state, ctx)
                .await
                .unwrap(),
            "portals",
            ctx,
        );
        self.tracing_guard
            .set(tracing_guard)
            .expect("Failed to initialize logs");
    }
}
