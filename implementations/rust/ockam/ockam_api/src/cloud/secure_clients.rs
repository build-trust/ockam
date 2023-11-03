use std::str::FromStr;
use std::time::Duration;
use tokio::spawn;

use ockam::identity::{Identifier, SecureChannel, SecureChannels, SecureClient, DEFAULT_TIMEOUT};
use ockam_core::compat::sync::Arc;
use ockam_core::env::{get_env, get_env_with_default, FromString};
use ockam_core::errcode::Kind;
use ockam_core::{AsyncTryClone, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnection, TcpTransport};

use crate::error::ApiError;
use crate::nodes::NodeManager;
use crate::{multiaddr_to_route, MultiAddrToRouteResult};

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// A default timeout
pub const ORCHESTRATOR_RESTART_TIMEOUT: Duration = Duration::from_secs(180);

/// Total time to wait for Orchestrator long-running operations to complete
pub const ORCHESTRATOR_AWAIT_TIMEOUT: Duration = Duration::from_secs(60);

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
    ) -> Result<GenericSecureClient> {
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
        let mut controller_route = Self::controller_route(tcp_transport).await?;
        let controller_identifier = Self::load_controller_identifier()?;

        let tcp_connection = if let Some(tcp_connection) = controller_route.tcp_connection.take() {
            Some((tcp_connection, tcp_transport.ctx().async_try_clone().await?))
        } else {
            None
        };

        Ok(Controller {
            secure_client: SecureClient::new(
                secure_channels,
                controller_route.route,
                &controller_identifier,
                caller_identifier,
                ORCHESTRATOR_RESTART_TIMEOUT,
            ),
            tcp_connection,
        })
    }

    pub async fn authority_node(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        authority_identifier: &Identifier,
        authority_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<AuthorityNode> {
        let mut authority_route =
            Self::resolve_secure_route(tcp_transport, authority_multiaddr).await?;

        let tcp_connection = if let Some(tcp_connection) = authority_route.tcp_connection.take() {
            Some((tcp_connection, tcp_transport.ctx().async_try_clone().await?))
        } else {
            None
        };

        Ok(AuthorityNode {
            secure_client: SecureClient::new(
                secure_channels,
                authority_route.route,
                authority_identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
            ),
            tcp_connection,
        })
    }

    pub async fn project_node(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<ProjectNode> {
        let mut project_route =
            Self::resolve_secure_route(tcp_transport, project_multiaddr).await?;

        let tcp_connection = if let Some(tcp_connection) = project_route.tcp_connection.take() {
            Some((tcp_connection, tcp_transport.ctx().async_try_clone().await?))
        } else {
            None
        };

        Ok(ProjectNode {
            secure_client: SecureClient::new(
                secure_channels,
                project_route.route,
                project_identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
            ),
            tcp_connection,
        })
    }

    pub async fn generic(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        identifier: &Identifier,
        multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<GenericSecureClient> {
        let mut route = Self::resolve_secure_route(tcp_transport, multiaddr).await?;

        let tcp_connection = if let Some(tcp_connection) = route.tcp_connection.take() {
            Some((tcp_connection, tcp_transport.ctx().async_try_clone().await?))
        } else {
            None
        };

        Ok(GenericSecureClient {
            secure_client: SecureClient::new(
                secure_channels,
                route.route,
                identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
            ),
            tcp_connection,
        })
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

    async fn controller_route(tcp_transport: &TcpTransport) -> Result<MultiAddrToRouteResult> {
        Self::resolve_secure_route(tcp_transport, &Self::controller_multiaddr()).await
    }

    async fn resolve_secure_route(
        tcp_transport: &TcpTransport,
        multiaddr: &MultiAddr,
    ) -> Result<MultiAddrToRouteResult> {
        let resolved = multiaddr_to_route(multiaddr, tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: multiaddr={multiaddr}"
                ))
            })?;
        debug!("using the secure route {}", resolved.route);
        Ok(resolved)
    }
}

pub struct AuthorityNode {
    pub(crate) secure_client: SecureClient,
    pub(crate) tcp_connection: Option<(TcpConnection, Context)>,
}
pub struct ProjectNode {
    pub(crate) secure_client: SecureClient,
    pub(crate) tcp_connection: Option<(TcpConnection, Context)>,
}
pub struct Controller {
    pub(crate) secure_client: SecureClient,
    pub(crate) tcp_connection: Option<(TcpConnection, Context)>,
}

pub struct GenericSecureClient {
    pub(crate) secure_client: SecureClient,
    pub(crate) tcp_connection: Option<(TcpConnection, Context)>,
}

pub trait HasSecureClient {
    fn get_secure_client(&self) -> &SecureClient;
}

impl HasSecureClient for AuthorityNode {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for ProjectNode {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for Controller {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for GenericSecureClient {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl AuthorityNode {
    pub async fn create_secure_channel(&self, ctx: &Context) -> Result<SecureChannel> {
        self.secure_client.create_secure_channel(ctx).await
    }

    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.secure_client.check_secure_channel(ctx).await
    }
}

impl ProjectNode {
    pub async fn create_secure_channel(&self, ctx: &Context) -> Result<SecureChannel> {
        self.secure_client.create_secure_channel(ctx).await
    }

    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.secure_client.check_secure_channel(ctx).await
    }
}

impl Drop for AuthorityNode {
    fn drop(&mut self) {
        if let Some((tcp_connection, context)) = self.tcp_connection.take() {
            spawn(async move {
                if let Err(err) = tcp_connection.stop(&context).await {
                    if err.code().kind != Kind::NotFound {
                        warn!("Failed to stop TCP connection: {}", err);
                    }
                }
            });
        }
    }
}

impl Drop for ProjectNode {
    fn drop(&mut self) {
        if let Some((tcp_connection, context)) = self.tcp_connection.take() {
            spawn(async move {
                if let Err(err) = tcp_connection.stop(&context).await {
                    if err.code().kind != Kind::NotFound {
                        warn!("Failed to stop TCP connection: {}", err);
                    }
                }
            });
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        if let Some((tcp_connection, context)) = self.tcp_connection.take() {
            spawn(async move {
                if let Err(err) = tcp_connection.stop(&context).await {
                    if err.code().kind != Kind::NotFound {
                        warn!("Failed to stop TCP connection: {}", err);
                    }
                }
            });
        }
    }
}

impl Drop for GenericSecureClient {
    fn drop(&mut self) {
        if let Some((tcp_connection, context)) = self.tcp_connection.take() {
            spawn(async move {
                if let Err(err) = tcp_connection.stop(&context).await {
                    if err.code().kind != Kind::NotFound {
                        warn!("Failed to stop TCP connection: {}", err);
                    }
                }
            });
        }
    }
}
