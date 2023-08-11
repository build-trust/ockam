use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;
use tracing::error;

pub(crate) async fn load_model_state(
    context: Arc<Context>,
    node_manager_worker: &NodeManagerWorker,
    cli_state: &CliState,
) {
    let _ = super::create_relay_impl(&context, cli_state, node_manager_worker)
        .await
        .map_err(|e| error!(?e, "failed to create relay at the default project"));
}
