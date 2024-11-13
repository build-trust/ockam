use std::ops::Deref;
use std::time::Duration;

use futures::executor;
use miette::IntoDiagnostic;

use ockam::identity::models::ChangeHistory;
use ockam::identity::{Identifier, SecureChannels};
use ockam::tcp::{TcpListenerOptions, TcpTransport};
use ockam::{Context, Result};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_core::errcode::Kind;
use ockam_multiaddr::MultiAddr;

use crate::cli_state::random_name;
use crate::cli_state::CliState;
use crate::cloud::project::Project;
use crate::cloud::{AuthorityNodeClient, ControllerClient, CredentialsEnabled, ProjectNodeClient};
use crate::nodes::models::transport::Port;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use crate::nodes::{NodeManager, NODEMANAGER_ADDR};

/// An `InMemoryNode` represents a full running node
/// In addition to a `NodeManager`, which is used to handle all the entities related to a node
/// (inlet/outlet, secure channels, etc...)
/// the in memory node also handles the supervisions of the node with other nodes
///
/// You need to use an InMemoryNode if:
///
///  - you want to start a full node in the current process with services: inlets, outlets, secure channels etc...
///  - you want to create a client to send requests to the controller, with the `create_controller` method
///  - you want to create a client to send requests to the project node, with the `create_project_client` method
///  - you want to create a client to send requests to the authority node, with the `create_authority_client` method
///
///
pub struct InMemoryNode {
    pub(crate) node_manager: Arc<NodeManager>,
    persistent: bool,
    /// Optional timeout duration for establishing secure channels and awaiting responses
    timeout: Option<Duration>,
}

/// This Deref instance makes it easy to access the NodeManager functions from an InMemoryNode
impl Deref for InMemoryNode {
    type Target = Arc<NodeManager>;

    fn deref(&self) -> &Self::Target {
        &self.node_manager
    }
}

impl Drop for InMemoryNode {
    fn drop(&mut self) {
        // Most of the InMemoryNodes should clean-up their resources when their process
        // stops. Except if they have been started with the `ockam node create` command
        // because in that case they can be restarted
        if !self.persistent {
            executor::block_on(async {
                let result = self.cli_state.remove_node(&self.node_name).await;
                if let Err(err) = result {
                    // code: 1032 maps to SQLITE_READONLY_DBMOVED - meaning the database has been
                    // moved to another directory, most likely already deleted
                    if !err.to_string().contains("code: 1032") {
                        error!("Cannot delete the node {}: {err:?}", self.node_name);
                    }
                }
            });
        }
    }
}

impl InMemoryNode {
    /// Start an in memory node
    pub async fn start(ctx: &Context, cli_state: &CliState) -> miette::Result<Self> {
        Self::start_with_project_name(ctx, cli_state, None).await
    }

    /// Start an in memory node with some project
    pub async fn start_with_project_name(
        ctx: &Context,
        cli_state: &CliState,
        project_name: Option<String>,
    ) -> miette::Result<Self> {
        let default_identity_name = cli_state
            .get_or_create_default_named_identity()
            .await?
            .name();
        Self::start_node(
            ctx,
            cli_state,
            &default_identity_name,
            None,
            project_name,
            None,
            None,
        )
        .await
    }

    /// Start an in memory node with some identity and project
    pub async fn start_with_identity_and_project_name(
        ctx: &Context,
        cli_state: &CliState,
        identity: Option<String>,
        project_name: Option<String>,
    ) -> miette::Result<Self> {
        let identity = cli_state.get_identity_name_or_default(&identity).await?;
        Self::start_node(ctx, cli_state, &identity, None, project_name, None, None).await
    }

    /// Start an in memory node with a specific identity
    pub async fn start_with_identity(
        ctx: &Context,
        cli_state: &CliState,
        identity: Option<String>,
    ) -> miette::Result<InMemoryNode> {
        let identity = cli_state.get_identity_name_or_default(&identity).await?;
        Self::start_node(ctx, cli_state, &identity, None, None, None, None).await
    }

    /// Start an in memory node
    #[instrument(name = "start in-memory node", skip_all)]
    pub async fn start_node(
        ctx: &Context,
        cli_state: &CliState,
        identity_name: &str,
        status_endpoint_port: Option<Port>,
        project_name: Option<String>,
        authority_identity: Option<ChangeHistory>,
        authority_route: Option<MultiAddr>,
    ) -> miette::Result<InMemoryNode> {
        let defaults = NodeManagerDefaults::default();

        let tcp = TcpTransport::create(ctx).into_diagnostic()?;
        let tcp_listener = tcp
            .listen(
                defaults.tcp_listener_address.as_str(),
                TcpListenerOptions::new(),
            )
            .await
            .into_diagnostic()?;

        let node = cli_state
            .start_node_with_optional_values(
                &defaults.node_name,
                &Some(identity_name.to_string()),
                &project_name,
                Some(&tcp_listener),
            )
            .await
            .into_diagnostic()?;

        let trust_options = cli_state
            .retrieve_trust_options(&project_name, &authority_identity, &authority_route, &None)
            .await
            .into_diagnostic()?;

        let node_manager = Self::new(
            ctx,
            NodeManagerGeneralOptions::new(
                cli_state.clone(),
                node.name(),
                false,
                status_endpoint_port,
                false,
            ),
            NodeManagerTransportOptions::new_tcp(tcp_listener.flow_control_id().clone(), tcp),
            trust_options,
        )
        .await
        .into_diagnostic()?;
        ctx.flow_controls()
            .add_consumer(&NODEMANAGER_ADDR.into(), tcp_listener.flow_control_id());
        Ok(node_manager)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        for session in self.registry.inlets.values() {
            session.session.lock().await.stop().await;
        }

        for session in self.registry.relays.values() {
            session.session.lock().await.stop().await;
        }

        for addr in DefaultAddress::iter() {
            let result = ctx.stop_address(&addr.into());
            // when stopping we can safely ignore missing services
            if let Err(err) = result {
                if err.code().kind == Kind::NotFound {
                    continue;
                } else {
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    /// Create a new in memory node with various options
    #[instrument(name = "new in-memory node", skip_all, fields(node_name = general_options.node_name))]
    pub async fn new(
        ctx: &Context,
        general_options: NodeManagerGeneralOptions,
        transport_options: NodeManagerTransportOptions,
        trust_options: NodeManagerTrustOptions,
    ) -> Result<Self> {
        let persistent = general_options.persistent;
        let node_manager =
            NodeManager::create(ctx, general_options, transport_options, trust_options).await?;
        Ok(Self {
            node_manager,
            persistent,
            timeout: None,
        })
    }

    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
    }

    /// Return a Controller client to send requests to the Controller
    pub async fn create_controller(&self) -> miette::Result<ControllerClient> {
        let client = self.node_manager.create_controller().await?;
        if let Some(timeout) = self.timeout {
            Ok(client
                .with_request_timeout(&timeout)
                .with_secure_channel_timeout(&timeout))
        } else {
            Ok(client)
        }
    }

    pub async fn create_authority_client_with_project(
        &self,
        ctx: &Context,
        project: &Project,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNodeClient> {
        let client = self
            .node_manager
            .create_authority_client_with_project(ctx, project, caller_identity_name)
            .await?;
        if let Some(timeout) = self.timeout {
            Ok(client
                .with_request_timeout(&timeout)
                .with_secure_channel_timeout(&timeout))
        } else {
            Ok(client)
        }
    }

    pub async fn create_authority_client_with_authority(
        &self,
        ctx: &Context,
        authority_identifier: &Identifier,
        authority_route: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNodeClient> {
        let client = self
            .node_manager
            .create_authority_client_with_authority(
                ctx,
                authority_identifier,
                authority_route,
                caller_identity_name,
            )
            .await?;
        if let Some(timeout) = self.timeout {
            Ok(client
                .with_request_timeout(&timeout)
                .with_secure_channel_timeout(&timeout))
        } else {
            Ok(client)
        }
    }

    pub async fn create_project_client(
        &self,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identity_name: Option<String>,
        credentials_enabled: CredentialsEnabled,
    ) -> miette::Result<ProjectNodeClient> {
        let client = self
            .node_manager
            .create_project_client(
                project_identifier,
                project_multiaddr,
                caller_identity_name,
                credentials_enabled,
            )
            .await?;
        if let Some(timeout) = self.timeout {
            Ok(client
                .with_request_timeout(&timeout)
                .with_secure_channel_timeout(&timeout))
        } else {
            Ok(client)
        }
    }
}

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: random_name(),
            tcp_listener_address: "127.0.0.1:0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ockam::test]
    async fn test_start_twice(ctx: &mut Context) -> Result<()> {
        let cli = CliState::test().await?;

        let node_manager1 = InMemoryNode::start(ctx, &cli).await;
        assert!(node_manager1.is_ok());

        let node_manager2 = InMemoryNode::start(ctx, &cli).await;
        if let Err(e) = node_manager2 {
            panic!("cannot start the node manager a second time: {e:?}");
        }
        Ok(())
    }
}
