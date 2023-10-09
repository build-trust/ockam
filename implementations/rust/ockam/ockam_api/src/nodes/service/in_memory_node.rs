use miette::IntoDiagnostic;
use std::ops::Deref;
use std::path::PathBuf;

use ockam::{Context, Result, TcpTransport};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_transport_tcp::TcpListenerOptions;

use crate::cli_state::random_name;
use crate::cli_state::{add_project_info_to_node_state, init_node_state, CliState};
use crate::cloud::Controller;
use crate::config::cli::TrustContextConfig;
use crate::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use crate::nodes::{NodeManager, NODEMANAGER_ADDR};
use crate::session::sessions::{Key, Session};
use crate::session::MedicHandle;
use crate::DefaultAddress;

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
    pub(crate) medic_handle: MedicHandle,
    persistent: bool,
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
            self.node_manager
                .delete_node()
                .unwrap_or_else(|_| panic!("cannot delete the node {}", self.node_name));
        }
    }
}

impl InMemoryNode {
    /// Start an in memory node
    pub async fn start(ctx: &Context, cli_state: &CliState) -> miette::Result<Self> {
        Self::start_with_trust_context(ctx, cli_state, None, None).await
    }

    /// Start an in memory node with some project and trust context data
    pub async fn start_with_trust_context(
        ctx: &Context,
        cli_state: &CliState,
        project_path: Option<&PathBuf>,
        trust_context_config: Option<TrustContextConfig>,
    ) -> miette::Result<Self> {
        Self::start_node(
            ctx,
            cli_state,
            None,
            None,
            project_path,
            trust_context_config,
        )
        .await
    }

    /// Start an in memory node
    pub async fn start_node(
        ctx: &Context,
        cli_state: &CliState,
        vault: Option<String>,
        identity: Option<String>,
        project_path: Option<&PathBuf>,
        trust_context_config: Option<TrustContextConfig>,
    ) -> miette::Result<InMemoryNode> {
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

        let node_manager = Self::new(
            ctx,
            NodeManagerGeneralOptions::new(
                cli_state.clone(),
                defaults.node_name.clone(),
                None,
                false,
                false,
            ),
            NodeManagerTransportOptions::new(listener.flow_control_id().clone(), tcp),
            NodeManagerTrustOptions::new(trust_context_config),
        )
        .await
        .into_diagnostic()?;
        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, listener.flow_control_id());
        Ok(node_manager)
    }

    /// Return a Controller client to send requests to the Controller
    pub async fn create_controller(&self) -> miette::Result<Controller> {
        self.create_controller_client().await.into_diagnostic()
    }

    pub fn add_session(&self, session: Session) -> Key {
        self.medic_handle.add_session(session)
    }

    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        self.medic_handle.stop_medic(ctx).await?;
        for addr in DefaultAddress::iter() {
            ctx.stop_worker(addr).await?;
        }
        Ok(())
    }

    /// Create a new in memory node with various options
    pub async fn new(
        ctx: &Context,
        general_options: NodeManagerGeneralOptions,
        transport_options: NodeManagerTransportOptions,
        trust_options: NodeManagerTrustOptions,
    ) -> Result<Self> {
        let persistent = general_options.persistent;
        let node_manager =
            NodeManager::create(ctx, general_options, transport_options, trust_options).await?;
        debug!("start the Medic");
        let medic_handle = MedicHandle::start_medic(ctx).await?;
        Ok(Self {
            node_manager: Arc::new(node_manager),
            medic_handle,
            persistent,
        })
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
