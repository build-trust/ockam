use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::service::SupervisedNodeManager;
use std::sync::Arc;

pub(crate) fn load_model_state(
    context: Arc<Context>,
    node_manager: Arc<SupervisedNodeManager>,
    cli_state: &CliState,
) {
    let cli_state = cli_state.clone();
    tauri::async_runtime::spawn(async move {
        super::create_relay(context, cli_state, node_manager.clone()).await;
    });
}
