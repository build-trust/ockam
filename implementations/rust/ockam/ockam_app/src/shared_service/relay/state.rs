use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;

pub(crate) async fn load_model_state(
    context: Arc<Context>,
    node_manager_worker: &NodeManagerWorker,
    cli_state: &CliState,
) {
    let node_manager_worker = node_manager_worker.clone();
    let cli_state = cli_state.clone();
    tauri::async_runtime::spawn(async move {
        super::create_relay(context, cli_state, node_manager_worker).await;
    });
}
