use crate::app::NODE_NAME;
use crate::Result;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::nodes::NodeManagerWorker;
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, trace, warn};

/// Try to create a relay until it succeeds
/// Once it's created, a `Medic` worker will monitor it and recreate it whenever it's unresponsive
pub async fn create_relay(
    context: Arc<Context>,
    cli_state: CliState,
    node_manager_worker: NodeManagerWorker,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            if create_relay_impl(&context, &cli_state, &node_manager_worker)
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}

/// Create a relay at the default project if doesn't exist yet
async fn create_relay_impl(
    context: &Context,
    cli_state: &CliState,
    node_manager_worker: &NodeManagerWorker,
) -> Result<Option<ForwarderInfo>> {
    trace!("Creating relay");
    if !cli_state.is_enrolled().unwrap_or(false) {
        trace!("User is not enrolled, skipping...");
        return Ok(None);
    }
    match cli_state.projects.default() {
        Ok(project) => {
            let relays = node_manager_worker.get_forwarders().await;
            if let Some(relay) = relays
                .iter()
                .find(|r| r.remote_address() == format!("forward_to_{NODE_NAME}"))
                .cloned()
            {
                debug!(project = %project.name(), "Relay already exists");
                Ok(Some(relay.clone()))
            } else {
                debug!(project = %project.name(), "Creating relay at project");
                let project_route = format!("/project/{}", project.name());
                let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
                let req = CreateForwarder::at_project(
                    project_address.clone(),
                    Some(NODE_NAME.to_string()),
                );
                let relay = node_manager_worker
                    .create_forwarder(context, req)
                    .await
                    .into_diagnostic()?;
                info!(forwarding_route = %relay.forwarding_route(), "Relay created at project");
                Ok(Some(relay))
            }
        }
        Err(err) => {
            warn!(%err, "No default project has ben set");
            Ok(None)
        }
    }
}
