use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::NodeManagerWorker;
use std::sync::Arc;
use tokio_retry::strategy::FixedInterval;
use tokio_retry::Retry;
use tracing::error;

pub(crate) async fn load_model_state(
    context: Arc<Context>,
    node_manager_worker: &NodeManagerWorker,
    cli_state: &CliState,
) {
    let node_manager_worker = node_manager_worker.clone();
    let cli_state = cli_state.clone();
    tauri::async_runtime::spawn(async move {
        let retry_strategy = FixedInterval::from_millis(60_000).take(10);
        let _ = Retry::spawn(retry_strategy.clone(), || async {
            super::create_relay_impl(&context, &cli_state, &node_manager_worker).await
        })
        .await
        .map_err(|e| error!(?e, "failed to create relay at the default project"));
    });
}
