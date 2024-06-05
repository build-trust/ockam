use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use ockam::identity::{
    CredentialRetrieverCreator, Identifier, SecureChannels, SecureClient, DEFAULT_TIMEOUT,
};
use ockam::tcp::TcpTransport;
use ockam_core::compat::sync::Arc;
use ockam_core::env::{get_env, get_env_with_default, FromString};
use ockam_core::{Result, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;

use crate::error::ApiError;
use crate::multiaddr_to_transport_route;
use crate::nodes::NodeManager;

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

/// If it's present, its contents will be used and will have priority over the contents
/// from ./static/controller.id.
/// How to use: when running a command that spawns a background node or use an embedded node
/// add the env variable. `OCKAM_CONTROLLER_IDENTITY_ID={identity.id-contents} ockam ...`
pub(crate) const OCKAM_CONTROLLER_IDENTITY_ID: &str = "OCKAM_CONTROLLER_IDENTITY_ID";

/// Total time to wait for Orchestrator long-running operations to complete
pub const ORCHESTRATOR_AWAIT_TIMEOUT: Duration = Duration::from_secs(60 * 10);

pub enum CredentialsEnabled {
    On,
    Off,
}

impl Display for CredentialsEnabled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialsEnabled::On => f.write_str("on"),
            CredentialsEnabled::Off => f.write_str("off"),
        }
    }
}

impl NodeManager {
    #[instrument(skip_all)]
    pub(crate) async fn create_controller_client(
        &self,
        timeout: Option<Duration>,
    ) -> Result<ControllerClient> {
        NodeManager::controller_node_client(
            &self.tcp_transport,
            self.secure_channels.clone(),
            &self.identifier(),
            timeout,
        )
        .await
    }

    #[instrument(skip_all, fields(authority_identifier = %authority_identifier.clone(), authority_route = %authority_route.clone(), caller = %caller_identifier.clone()))]
    pub(crate) async fn make_authority_node_client(
        &self,
        authority_identifier: &Identifier,
        authority_route: &MultiAddr,
        caller_identifier: &Identifier,
        credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
    ) -> Result<AuthorityNodeClient> {
        NodeManager::authority_node_client(
            &self.tcp_transport,
            self.secure_channels.clone(),
            authority_identifier,
            authority_route,
            caller_identifier,
            credential_retriever_creator,
        )
        .await
    }

    #[instrument(skip_all, fields(project_identifier = %project_identifier.clone(), project_multiaddr = %project_multiaddr.clone(), caller = %caller_identifier.clone(), credentials_enabled = %credentials_enabled))]
    pub(crate) async fn make_project_node_client(
        &self,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
        // TODO: Currently admin authenticates as a member on the Project node, but we may choose to
        //  use project admin credentials in the future
        credentials_enabled: CredentialsEnabled,
    ) -> Result<ProjectNodeClient> {
        let credential_retriever_creator = match credentials_enabled {
            CredentialsEnabled::On => self.credential_retriever_creators.project_member.clone(),
            CredentialsEnabled::Off => None,
        };

        NodeManager::project_node_client(
            &self.tcp_transport,
            self.secure_channels.clone(),
            credential_retriever_creator,
            project_identifier,
            project_multiaddr,
            caller_identifier,
        )
        .await
    }

    #[instrument(skip_all, fields(identifier = %identifier.clone(), multiaddr = %multiaddr.clone(), caller = %caller_identifier.clone()))]
    pub async fn make_secure_client(
        &self,
        identifier: &Identifier,
        multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<GenericSecureClient> {
        NodeManager::generic_client(
            &self.tcp_transport,
            self.secure_channels.clone(),
            identifier,
            multiaddr,
            caller_identifier,
        )
        .await
    }

    #[instrument(skip_all, fields(caller = %caller_identifier.clone()))]
    pub async fn controller_node_client(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        caller_identifier: &Identifier,
        timeout: Option<Duration>,
    ) -> Result<ControllerClient> {
        let controller_route = Self::controller_route().await?;
        let controller_identifier = Self::load_controller_identifier()?;

        Ok(ControllerClient {
            secure_client: SecureClient::new(
                secure_channels,
                None,
                Arc::new(tcp_transport.clone()),
                controller_route,
                &controller_identifier,
                caller_identifier,
                timeout.unwrap_or(DEFAULT_TIMEOUT),
                timeout.unwrap_or(DEFAULT_TIMEOUT),
            ),
        })
    }

    #[instrument(skip_all, fields(authority_identifier = %authority_identifier.clone(), authority_route = %authority_route.clone(), caller = %caller_identifier.clone()))]
    pub async fn authority_node_client(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        authority_identifier: &Identifier,
        authority_route: &MultiAddr,
        caller_identifier: &Identifier,
        credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
    ) -> Result<AuthorityNodeClient> {
        let authority_route = multiaddr_to_transport_route(authority_route).ok_or_else(|| {
            ApiError::core(format!(
                "Couldn't convert MultiAddr to route: multiaddr={authority_route}"
            ))
        })?;

        Ok(AuthorityNodeClient {
            secure_client: SecureClient::new(
                secure_channels,
                credential_retriever_creator,
                Arc::new(tcp_transport.clone()),
                authority_route,
                authority_identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
                DEFAULT_TIMEOUT,
            ),
        })
    }

    #[instrument(skip_all, fields(project_identifier = %project_identifier.clone(), project_multiaddr = %project_multiaddr.clone(), caller = %caller_identifier.clone()))]
    pub async fn project_node_client(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        credential_retriever_creator: Option<Arc<dyn CredentialRetrieverCreator>>,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<ProjectNodeClient> {
        let project_route = multiaddr_to_transport_route(project_multiaddr).ok_or_else(|| {
            ApiError::core(format!(
                "Couldn't convert MultiAddr to route: multiaddr={project_multiaddr}"
            ))
        })?;

        Ok(ProjectNodeClient {
            secure_client: SecureClient::new(
                secure_channels,
                credential_retriever_creator,
                Arc::new(tcp_transport.clone()),
                project_route,
                project_identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
                DEFAULT_TIMEOUT,
            ),
        })
    }

    pub async fn generic_client(
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
        identifier: &Identifier,
        multiaddr: &MultiAddr,
        caller_identifier: &Identifier,
    ) -> Result<GenericSecureClient> {
        let route = multiaddr_to_transport_route(multiaddr).ok_or_else(|| {
            ApiError::core(format!(
                "Couldn't convert MultiAddr to route: multiaddr={multiaddr}"
            ))
        })?;

        Ok(GenericSecureClient {
            secure_client: SecureClient::new(
                secure_channels,
                None,
                Arc::new(tcp_transport.clone()),
                route,
                identifier,
                caller_identifier,
                DEFAULT_TIMEOUT,
                DEFAULT_TIMEOUT,
            ),
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

    pub async fn controller_route() -> Result<Route> {
        let multiaddr = Self::controller_multiaddr();
        multiaddr_to_transport_route(&multiaddr).ok_or_else(|| {
            ApiError::core(format!(
                "Couldn't convert MultiAddr to route: multiaddr={multiaddr}"
            ))
        })
    }
}

pub struct AuthorityNodeClient {
    secure_client: SecureClient,
}

pub struct ProjectNodeClient {
    secure_client: SecureClient,
}

pub struct ControllerClient {
    secure_client: SecureClient,
}

pub struct GenericSecureClient {
    secure_client: SecureClient,
}

pub trait HasSecureClient {
    fn get_secure_client(&self) -> &SecureClient;
}

impl HasSecureClient for AuthorityNodeClient {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for ProjectNodeClient {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for ControllerClient {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl HasSecureClient for GenericSecureClient {
    fn get_secure_client(&self) -> &SecureClient {
        &self.secure_client
    }
}

impl AuthorityNodeClient {
    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.secure_client.check_secure_channel(ctx).await
    }

    pub fn new(secure_client: SecureClient) -> Self {
        Self { secure_client }
    }
}

impl ProjectNodeClient {
    pub async fn check_secure_channel(&self, ctx: &Context) -> Result<()> {
        self.secure_client.check_secure_channel(ctx).await
    }

    pub fn new(secure_client: SecureClient) -> Self {
        Self { secure_client }
    }
}

impl ControllerClient {
    pub fn new(secure_client: SecureClient) -> Self {
        Self { secure_client }
    }
}

impl GenericSecureClient {
    pub fn new(secure_client: SecureClient) -> Self {
        Self { secure_client }
    }
}
