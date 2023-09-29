use std::str::FromStr;
use std::time::Duration;

use ockam::identity::{Identifier, SecureChannel, SecureChannels, SecureClient, DEFAULT_TIMEOUT};
use ockam_core::compat::sync::Arc;
use ockam_core::env::{get_env, get_env_with_default, FromString};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

use crate::error::ApiError;
use crate::multiaddr_to_route;
use crate::nodes::NodeManager;

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// A default timeout in seconds
pub const ORCHESTRATOR_RESTART_TIMEOUT: u64 = 180;

/// Total time in milliseconds to wait for Orchestrator long-running operations to complete
pub const ORCHESTRATOR_AWAIT_TIMEOUT_MS: usize = 60 * 10 * 1000;

impl NodeManager {
    pub(crate) async fn create_controller_client(&self) -> Result<Controller> {
        NodeManager::controller_node(
            &self.tcp_transport,
            self.secure_channels.clone(),
            &self.get_identifier(None).await?,
        )
        .await
    }

    pub(crate) async fn make_authority_node_client(
        &self,
        authority_identifier: &Identifier,
        authority_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<AuthorityNode> {
        NodeManager::authority_node(
            &self.tcp_transport,
            self.secure_channels.clone(),
            authority_identifier,
            authority_multiaddr,
            caller_identifier,
        )
        .await
    }

    pub(crate) async fn make_project_node_client(
        &self,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<ProjectNode> {
        NodeManager::project_node(
            &self.tcp_transport,
            self.secure_channels.clone(),
            project_identifier,
            project_multiaddr,
            caller_identifier,
        )
        .await
    }

    pub async fn make_secure_client(
        &self,
        identifier: &Identifier,
        multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<SecureClient> {
        NodeManager::generic(
            &self.tcp_transport,
            self.secure_channels.clone(),
            identifier,
            multiaddr,
            caller_identifier,
        )
        .await
    }

    pub async fn controller_node(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        caller_identifier: &Identifier,
    ) -> Result<Controller> {
        let controller_route = Self::controller_route(tcp_transport).await?;
        let controller_identifier = Self::load_controller_identifier()?;

        Ok(Controller(SecureClient::new(
            secure_channels,
            controller_route,
            &controller_identifier,
            caller_identifier,
            Duration::from_secs(ORCHESTRATOR_RESTART_TIMEOUT),
        )))
    }

    pub async fn authority_node(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        authority_identifier: &Identifier,
        authority_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<AuthorityNode> {
        let authority_route =
            Self::resolve_secure_route(tcp_transport, authority_multiaddr).await?;

        Ok(AuthorityNode(SecureClient::new(
            secure_channels,
            authority_route,
            authority_identifier,
            caller_identifier,
            DEFAULT_TIMEOUT,
        )))
    }

    pub async fn project_node(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<ProjectNode> {
        let project_route = Self::resolve_secure_route(tcp_transport, project_multiaddr).await?;

        Ok(ProjectNode(SecureClient::new(
            secure_channels,
            project_route,
            project_identifier,
            caller_identifier,
            DEFAULT_TIMEOUT,
        )))
    }

    pub async fn generic(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        identifier: &Identifier,
        multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<SecureClient> {
        let route = Self::resolve_secure_route(tcp_transport, multiaddr).await?;

        Ok(SecureClient::new(
            secure_channels,
            route,
            identifier,
            caller_identifier,
            DEFAULT_TIMEOUT,
        ))
    }

    /// Load controller identity id from file.
    /// If the env var `OCKAM_CONTROLLER_IDENTITY_ID` is set, that will be used to
    /// load the identifier instead of the file.
    pub fn load_controller_identifier() -> Result<Identifier> {
        if let Ok(Some(idt)) = get_env::<Identifier>(OCKAM_CONTROLLER_IDENTITY_ID) {
            trace!(idt = %idt, "Read controller identifier from env");
            return Ok(idt);
        }
        Identifier::from_str(include_str!("../../static/controller.id"))
    }

    pub fn controller_multiaddr() -> MultiAddr {
        let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
            .unwrap_or_else(|_| panic!("invalid Controller address: {DEFAULT_CONTROLLER_ADDRESS}"));
        get_env_with_default::<MultiAddr>(OCKAM_CONTROLLER_ADDR, default_addr).unwrap()
    }

    async fn controller_route(tcp_transport: &TcpTransport) -> Result<Route> {
        Self::resolve_secure_route(tcp_transport, &Self::controller_multiaddr()).await
    }

    async fn resolve_secure_route(
        tcp_transport: &TcpTransport,
        multiaddr: &MultiAddr,
    ) -> Result<Route> {
        let secure_route = multiaddr_to_route(multiaddr, tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: multiaddr={multiaddr}"
                ))
            })?
            .route;
        debug!("using the secure route {secure_route}");
        Ok(secure_route)
    }
}

pub struct AuthorityNode(pub(crate) SecureClient);
pub struct ProjectNode(pub(crate) SecureClient);
pub struct Controller(pub(crate) SecureClient);

pub trait HasSecureClient {
    fn get_secure_client(&self) -> &SecureClient;
}

impl HasSecureClient for AuthorityNode {
    fn get_secure_client(&self) -> &SecureClient {
        &self.0
    }
}

impl HasSecureClient for ProjectNode {
    fn get_secure_client(&self) -> &SecureClient {
        &self.0
    }
}

impl HasSecureClient for Controller {
    fn get_secure_client(&self) -> &SecureClient {
        &self.0
    }
}

impl AuthorityNode {
    pub async fn create_secure_channel(&self, ctx: &Context) -> Result<SecureChannel> {
        self.0.create_secure_channel(ctx).await
    }

    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.0.check_secure_channel(ctx).await
    }
}

impl ProjectNode {
    pub async fn create_secure_channel(&self, ctx: &Context) -> Result<SecureChannel> {
        self.0.create_secure_channel(ctx).await
    }

    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.0.check_secure_channel(ctx).await
    }
}
