use crate::app::NODE_NAME;
use crate::Result;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::nodes::NodeManagerWorker;
use ockam_multiaddr::MultiAddr;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, trace, warn};

pub static RELAY_NAME: Lazy<String> = Lazy::new(|| format!("forward_to_{NODE_NAME}"));

/// Try to create a relay until it succeeds.
pub async fn create_relay(
    context: Arc<Context>,
    cli_state: CliState,
    node_manager_worker: NodeManagerWorker,
) {
    loop {
        match create_relay_impl(&context, &cli_state, &node_manager_worker).await {
            Ok(_) => break,
            Err(e) => {
                warn!(%e, "Failed to create relay, retrying...");
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

/// Create a relay at the default project if doesn't exist yet
///
/// Once it's created, a `Medic` worker will monitor it and recreate it whenever it's unresponsive
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
            if let Some(relay) = get_relay(node_manager_worker).await {
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

pub(crate) async fn get_relay(node_manager_worker: &NodeManagerWorker) -> Option<ForwarderInfo> {
    node_manager_worker
        .get_forwarders()
        .await
        .into_iter()
        .find(|r| r.remote_address() == *RELAY_NAME)
}
