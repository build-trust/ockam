use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use miette::IntoDiagnostic;
use rand::random;

use ockam::identity::{Identifier, SecureClient};
use ockam::{Context, TcpListenerOptions, TcpTransport};
use ockam_core::async_trait;
use ockam_multiaddr::MultiAddr;

use crate::cli_state::{add_project_info_to_node_state, init_node_state, CliState};
use crate::cloud::{AuthorityNode, Controller, ProjectNode};
use crate::config::cli::TrustContextConfig;

use crate::nodes::service::message::MessageSender;
use crate::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
    SupervisedNodeManager,
};
use crate::nodes::NODEMANAGER_ADDR;

/// This struct represents a node that lives within the current process
pub struct InMemoryNode {
    cli_state: CliState,
    node_manager: SupervisedNodeManager,
    controller: Arc<Controller>,
}

impl InMemoryNode {
    pub async fn create(
        ctx: &Context,
        cli_state: &CliState,
        project_path: Option<&PathBuf>,
        trust_context_config: Option<TrustContextConfig>,
    ) -> miette::Result<InMemoryNode> {
        let node_manager =
            start_node_manager(ctx, cli_state, project_path, trust_context_config).await?;
        let controller = node_manager
            .make_controller_node_client()
            .await
            .into_diagnostic()?;
        Ok(Self {
            cli_state: cli_state.clone(),
            node_manager,
            controller: Arc::new(controller),
        })
    }

    pub async fn make_project_node_client(
        &self,
        project_identifier: &Identifier,
        project_address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<ProjectNode> {
        self.node_manager
            .make_project_node_client(
                project_identifier,
                project_address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub async fn make_authority_node_client(
        &self,
        authority_identifier: &Identifier,
        authority_address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNode> {
        self.node_manager
            .make_authority_node_client(
                authority_identifier,
                authority_address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub async fn make_secure_client(
        &self,
        identifier: &Identifier,
        address: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<SecureClient> {
        self.node_manager
            .make_secure_client(
                identifier,
                address,
                &self
                    .node_manager
                    .get_identifier(caller_identity_name)
                    .await
                    .into_diagnostic()?,
            )
            .await
            .into_diagnostic()
    }

    pub fn node_name(&self) -> String {
        self.node_manager.node_name()
    }
}

impl Deref for InMemoryNode {
    type Target = Arc<Controller>;

    fn deref(&self) -> &Self::Target {
        &self.controller
    }
}

impl Drop for InMemoryNode {
    fn drop(&mut self) {
        let _ = self
            .cli_state
            .nodes
            .delete_sigkill(self.node_manager.node_name().as_str(), false);
    }
}

#[async_trait]
impl MessageSender for InMemoryNode {
    async fn send_message(
        &self,
        ctx: &Context,
        addr: &MultiAddr,
        message: Vec<u8>,
    ) -> ockam_core::Result<Vec<u8>> {
        self.node_manager.send_message(ctx, addr, message).await
    }
}

pub struct NodeManagerDefaults {
    pub node_name: String,
    pub tcp_listener_address: String,
}

impl Default for NodeManagerDefaults {
    fn default() -> Self {
        Self {
            node_name: hex::encode(random::<[u8; 4]>()),
            tcp_listener_address: "127.0.0.1:0".to_string(),
        }
    }
}

pub async fn start_node_manager(
    ctx: &Context,
    cli_state: &CliState,
    project_path: Option<&PathBuf>,
    trust_context_config: Option<TrustContextConfig>,
) -> miette::Result<SupervisedNodeManager> {
    start_node_manager_with_vault_and_identity(
        ctx,
        cli_state,
        None,
        None,
        project_path,
        trust_context_config,
    )
    .await
}

pub async fn start_node_manager_with_vault_and_identity(
    ctx: &Context,
    cli_state: &CliState,
    vault: Option<String>,
    identity: Option<String>,
    project_path: Option<&PathBuf>,
    trust_context_config: Option<TrustContextConfig>,
) -> miette::Result<SupervisedNodeManager> {
    let defaults = NodeManagerDefaults::default();

    init_node_state(
        cli_state,
        &defaults.node_name,
        vault.as_deref(),
        identity.as_deref(),
    )
    .await?;

    add_project_info_to_node_state(&defaults.node_name, cli_state, project_path).await?;

    let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
    let bind = defaults.tcp_listener_address;

    let options = TcpListenerOptions::new();
    let listener = tcp.listen(&bind, options).await.into_diagnostic()?;

    let node_manager = SupervisedNodeManager::create(
        ctx,
        NodeManagerGeneralOptions::new(cli_state.clone(), defaults.node_name.clone(), false, None),
        NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
        NodeManagerTrustOptions::new(trust_context_config),
    )
    .await
    .into_diagnostic()?;
    ctx.flow_controls()
        .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
    Ok(node_manager)
}
