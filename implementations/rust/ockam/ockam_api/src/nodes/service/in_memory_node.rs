use std::ops::Deref;
use std::time::Duration;

use futures::executor;
use miette::IntoDiagnostic;

use ockam::identity::SecureChannels;
use ockam::{Context, Result, TcpTransport};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_core::errcode::Kind;
use ockam_transport_tcp::TcpListenerOptions;

use crate::cli_state::random_name;
use crate::cli_state::CliState;
use crate::cli_state::NamedTrustContext;
use crate::cloud::ControllerClient;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::service::{
    NodeManagerGeneralOptions, NodeManagerTransportOptions, NodeManagerTrustOptions,
};
use crate::nodes::{NodeManager, NODEMANAGER_ADDR};
use crate::session::sessions::Session;

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
                self.node_manager
                    .delete_node()
                    .await
                    .unwrap_or_else(|e| panic!("cannot delete the node {}: {e:?}", self.node_name))
            });
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
        project_name: Option<String>,
        trust_context: Option<NamedTrustContext>,
    ) -> miette::Result<Self> {
        let default_identity_name = cli_state
            .get_or_create_default_named_identity()
            .await?
            .name();
        Self::start_node(
            ctx,
            cli_state,
            &default_identity_name,
            project_name,
            trust_context,
        )
        .await
    }

    /// Start an in memory node with a specific identity
    pub async fn start_node_with_identity(
        ctx: &Context,
        cli_state: &CliState,
        identity_name: &str,
    ) -> miette::Result<InMemoryNode> {
        Self::start_node(ctx, cli_state, identity_name, None, None).await
    }

    /// Start an in memory node
    pub async fn start_node(
        ctx: &Context,
        cli_state: &CliState,
        identity_name: &str,
        project_name: Option<String>,
        trust_context: Option<NamedTrustContext>,
    ) -> miette::Result<InMemoryNode> {
        let defaults = NodeManagerDefaults::default();

        let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
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

        let node_manager = Self::new(
            ctx,
            NodeManagerGeneralOptions::new(cli_state.clone(), node.name(), None, false, false),
            NodeManagerTransportOptions::new(tcp_listener.flow_control_id().clone(), tcp),
            NodeManagerTrustOptions::new(trust_context),
        )
        .await
        .into_diagnostic()?;
        ctx.flow_controls()
            .add_consumer(NODEMANAGER_ADDR, tcp_listener.flow_control_id());
        Ok(node_manager)
    }

    /// Return a Controller client to send requests to the Controller
    pub async fn create_controller(&self) -> miette::Result<ControllerClient> {
        self.create_controller_client(self.timeout)
            .await
            .into_diagnostic()
    }

    pub fn add_session(&self, session: Session) {
        self.medic_handle.add_session(session);
    }

    pub fn remove_session(&self, key: &str) {
        self.medic_handle.remove_session(key);
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        self.medic_handle.stop_medic(ctx).await?;
        for addr in DefaultAddress::iter() {
            let result = ctx.stop_worker(addr).await;
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
        Ok(Self {
            node_manager: Arc::new(node_manager),
            persistent,
            timeout: None,
        })
    }

    pub fn secure_channels(&self) -> Arc<SecureChannels> {
        self.secure_channels.clone()
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
