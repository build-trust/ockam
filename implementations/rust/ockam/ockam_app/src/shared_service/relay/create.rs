use crate::app::{AppState, NODE_NAME};
use crate::Result;
use miette::IntoDiagnostic;
use ockam::Context;
use ockam_api::cli_state::{CliState, StateDirTrait};
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::nodes::NodeManagerWorker;
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;
use tracing::{debug, info};

pub async fn create_relay(app_state: &AppState) -> Result<()> {
    create_relay_impl(
        &app_state.context(),
        &app_state.state().await,
        &app_state.node_manager_worker().await,
    )
    .await?;
    Ok(())
}

/// Create a relay at the default project
pub async fn create_relay_impl(
    context: &Context,
    cli_state: &CliState,
    node_manager_worker: &NodeManagerWorker,
) -> Result<Option<ForwarderInfo>> {
    match cli_state.projects.default() {
        Ok(project) => {
            debug!(project = %project.name(), "Creating relay at project");
            let project_route = format!("/project/{}", project.name());
            let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
            let req =
                CreateForwarder::at_project(project_address.clone(), Some(NODE_NAME.to_string()));
            let relay = node_manager_worker
                .create_forwarder(context, req)
                .await
                .into_diagnostic()?;
            info!(forwarding_route = %relay.forwarding_route(), "Relay created at project");
            Ok(Some(relay))
        }
        Err(_) => Ok(None),
    }
}
