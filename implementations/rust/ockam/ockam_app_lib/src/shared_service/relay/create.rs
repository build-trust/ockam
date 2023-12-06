use std::str::FromStr;
use std::sync::Arc;

use miette::IntoDiagnostic;
use tracing::{debug, info, trace, warn};

use ockam::Context;
use ockam_api::cli_state::CliState;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::InMemoryNode;
use ockam_multiaddr::MultiAddr;

use crate::api::state::OrchestratorStatus;
use crate::state::AppState;
use crate::Result;

impl AppState {
    /// Try to create a relay until it succeeds.
    pub async fn refresh_relay(&self) {
        let cli_state = self.state().await;
        let node_manager = self.node_manager().await;
        let context = self.context();

        if !self.is_enrolled().await.unwrap_or(false) {
            // During the enrollment phase the status would be enrollment-related
            // and we don't want to overwrite it with disconnected
            self.update_orchestrator_status_if(
                OrchestratorStatus::Disconnected,
                vec![
                    OrchestratorStatus::Connected,
                    OrchestratorStatus::Connecting,
                ],
            );
            self.publish_state().await;

            debug!("Not enrolled, skipping relay creation");
            match get_relay(&node_manager, &cli_state).await {
                Ok(_) => match delete_relay(context, &node_manager, &cli_state).await {
                    Ok(_) => {
                        info!("Relay deleted");
                    }
                    Err(err) => {
                        warn!(%err, "Cannot delete relay")
                    }
                },
                Err(err) => {
                    warn!(%err, "Cannot get relay")
                }
            }
            return;
        }

        let result = self
            .create_relay_impl(&context, &cli_state, node_manager.clone())
            .await;

        if let Err(e) = result {
            warn!(%e, "Failed to create relay, retrying...");
        }
    }

    /// Create a relay at the default project if doesn't exist yet
    ///
    /// Once it's created, a `Medic` worker will monitor it and recreate it whenever it's unresponsive
    async fn create_relay_impl(
        &self,
        context: &Context,
        cli_state: &CliState,
        node_manager: Arc<InMemoryNode>,
    ) -> Result<()> {
        trace!("Creating relay");
        match cli_state.get_default_project().await {
            Ok(project) => {
                if let Some(_relay) = get_relay(&node_manager, cli_state).await? {
                    debug!(project = %project.name(), "Relay already exists");
                    self.update_orchestrator_status(OrchestratorStatus::Connected);
                    self.publish_state().await;
                    Ok(())
                } else {
                    self.update_orchestrator_status(OrchestratorStatus::Connecting);
                    self.publish_state().await;
                    debug!(project = %project.name(), "Creating relay at project");
                    let project_route = format!("/project/{}", project.name());
                    let project_address = MultiAddr::from_str(&project_route).into_diagnostic()?;
                    let relay = node_manager
                        .create_relay(
                            context,
                            &project_address,
                            Some(bare_relay_name(cli_state).await?),
                            false,
                            None,
                        )
                        .await
                        .into_diagnostic()?;
                    info!(forwarding_route = %relay.forwarding_route(), "Relay created at project");
                    self.update_orchestrator_status(OrchestratorStatus::Connected);
                    self.publish_state().await;
                    Ok(())
                }
            }
            Err(err) => {
                warn!(%err, "No default project has ben set");
                Ok(())
            }
        }
    }
}

async fn delete_relay(
    context: Arc<Context>,
    node_manager: &InMemoryNode,
    cli_state: &CliState,
) -> ockam::Result<Option<RelayInfo>> {
    let relay_name = relay_name(cli_state).await?;
    node_manager.delete_relay(&context, &relay_name).await
}

async fn get_relay(
    node_manager: &InMemoryNode,
    cli_state: &CliState,
) -> ockam::Result<Option<RelayInfo>> {
    let relay_name = relay_name(cli_state).await?;
    Ok(node_manager
        .get_relays()
        .await
        .into_iter()
        .find(|r| r.remote_address() == relay_name))
}

async fn relay_name(cli_state: &CliState) -> ockam::Result<String> {
    let bare_relay_name = bare_relay_name(cli_state).await?;
    Ok(format!("forward_to_{bare_relay_name}"))
}

async fn bare_relay_name(cli_state: &CliState) -> ockam::Result<String> {
    Ok(cli_state
        .get_or_create_default_named_identity()
        .await?
        .identifier()
        .to_string())
}
