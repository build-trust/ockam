use crate::cloud::project::Project;
use crate::cloud::{AuthorityNodeClient, CredentialsEnabled, ProjectNodeClient};
use crate::nodes::connection::{
    Connection, ConnectionBuilder, PlainTcpInstantiator, ProjectInstantiator,
    SecureChannelInstantiator,
};
use crate::nodes::models::portal::OutletStatus;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::registry::Registry;
use crate::nodes::service::http::HttpServer;
use crate::nodes::service::{
    CredentialRetrieverCreators, NodeManagerCredentialRetrieverOptions, NodeManagerTrustOptions,
    SecureChannelType,
};

use crate::session::MedicHandle;
use crate::{ApiError, CliState, DefaultAddress};
use miette::IntoDiagnostic;
use ockam::identity::{
    CachedCredentialRetrieverCreator, CredentialRetrieverCreator, Identifier,
    MemoryCredentialRetrieverCreator, RemoteCredentialRetrieverCreator, SecureChannels,
};
use ockam::tcp::TcpTransport;
use ockam::{RelayService, RelayServiceOptions};
use ockam_abac::expr::str;
use ockam_abac::{
    Action, Env, Policies, PolicyAccessControl, PolicyExpression, Resource, ResourceType, Resources,
};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{
    AllowAll, AsyncTryClone, CachedIncomingAccessControl, CachedOutgoingAccessControl,
    IncomingAccessControl, OutgoingAccessControl,
};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Node manager provides high-level operations to
///  - send messages
///  - create secure channels, inlet, outlet
///  - configure the trust
///  - manage persistent data
pub struct NodeManager {
    pub(crate) cli_state: CliState,
    pub(super) node_name: String,
    pub(super) node_identifier: Identifier,
    pub(super) api_transport_flow_control_id: FlowControlId,
    pub(crate) tcp_transport: TcpTransport,
    pub(crate) secure_channels: Arc<SecureChannels>,
    pub(crate) credential_retriever_creators: CredentialRetrieverCreators,
    pub(super) project_authority: Option<Identifier>,
    pub(crate) registry: Arc<Registry>,
    pub(crate) medic_handle: MedicHandle,
}

impl NodeManager {
    /// Create a new NodeManager with the node name from the ockam CLI
    #[instrument(name = "create_node_manager", skip_all, fields(node_name = general_options.node_name))]
    pub async fn create(
        ctx: &Context,
        general_options: NodeManagerGeneralOptions,
        transport_options: NodeManagerTransportOptions,
        trust_options: NodeManagerTrustOptions,
    ) -> ockam_core::Result<Arc<Self>> {
        let cli_state = general_options.cli_state;
        let node_name = general_options.node_name.clone();

        let registry = Arc::new(Registry::default());

        debug!("start the medic");
        let medic_handle = MedicHandle::start_medic(ctx, registry.clone()).await?;

        debug!("retrieve the node identifier");
        let node_identifier = cli_state.get_node(&node_name).await?.identifier();

        debug!("create default resource type policies");
        cli_state
            .policies(&general_options.node_name)
            .store_default_resource_type_policies()
            .await?;

        let secure_channels = cli_state.secure_channels(&node_name).await?;

        let project_member_credential_retriever_creator: Option<
            Arc<dyn CredentialRetrieverCreator>,
        > = match trust_options.project_member_credential_retriever_options {
            NodeManagerCredentialRetrieverOptions::None => None,
            NodeManagerCredentialRetrieverOptions::CacheOnly { issuer, scope } => {
                Some(Arc::new(CachedCredentialRetrieverCreator::new(
                    issuer.clone(),
                    scope,
                    secure_channels.identities().cached_credentials_repository(),
                )))
            }
            NodeManagerCredentialRetrieverOptions::Remote { info, scope } => {
                Some(Arc::new(RemoteCredentialRetrieverCreator::new(
                    ctx.async_try_clone().await?,
                    Arc::new(transport_options.tcp_transport.clone()),
                    secure_channels.clone(),
                    info.clone(),
                    scope,
                )))
            }
            NodeManagerCredentialRetrieverOptions::InMemory(credential) => {
                Some(Arc::new(MemoryCredentialRetrieverCreator::new(credential)))
            }
        };

        let project_admin_credential_retriever_creator: Option<
            Arc<dyn CredentialRetrieverCreator>,
        > = match trust_options.project_admin_credential_retriever_options {
            NodeManagerCredentialRetrieverOptions::None => None,
            NodeManagerCredentialRetrieverOptions::CacheOnly { issuer, scope } => {
                Some(Arc::new(CachedCredentialRetrieverCreator::new(
                    issuer.clone(),
                    scope,
                    secure_channels.identities().cached_credentials_repository(),
                )))
            }
            NodeManagerCredentialRetrieverOptions::Remote { info, scope } => {
                Some(Arc::new(RemoteCredentialRetrieverCreator::new(
                    ctx.async_try_clone().await?,
                    Arc::new(transport_options.tcp_transport.clone()),
                    secure_channels.clone(),
                    info.clone(),
                    scope,
                )))
            }
            NodeManagerCredentialRetrieverOptions::InMemory(credential) => {
                Some(Arc::new(MemoryCredentialRetrieverCreator::new(credential)))
            }
        };

        let credential_retriever_creators = CredentialRetrieverCreators {
            project_member: project_member_credential_retriever_creator,
            project_admin: project_admin_credential_retriever_creator,
            _account_admin: None,
        };

        let mut s = Self {
            cli_state,
            node_name,
            node_identifier,
            api_transport_flow_control_id: transport_options.api_transport_flow_control_id,
            tcp_transport: transport_options.tcp_transport,
            secure_channels,
            credential_retriever_creators,
            project_authority: trust_options.project_authority,
            registry,
            medic_handle,
        };

        debug!("initializing services");
        s.initialize_services(ctx, general_options.start_default_services)
            .await?;

        let s = Arc::new(s);

        if let Some(http_server_port) = general_options.http_server_port {
            debug!("start the http server");
            HttpServer::start(s.clone(), http_server_port)
                .await
                .map_err(|e| ApiError::core(e.to_string()))?;
        }

        info!("created a node manager for the node: {}", s.node_name);

        Ok(s)
    }

    async fn initialize_default_services(
        &self,
        ctx: &Context,
        api_flow_control_id: &FlowControlId,
    ) -> ockam_core::Result<()> {
        // Start services
        ctx.flow_controls()
            .add_consumer(DefaultAddress::UPPERCASE_SERVICE, api_flow_control_id);
        self.start_uppercase_service_impl(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
            .await?;

        let secure_channel_listener = self
            .create_secure_channel_listener(
                DefaultAddress::SECURE_CHANNEL_LISTENER.into(),
                None, // Not checking identifiers here in favor of credential check
                None,
                ctx,
                SecureChannelType::KeyExchangeAndMessages,
            )
            .await?;

        let options = RelayServiceOptions::new()
            .alias(DefaultAddress::STATIC_RELAY_SERVICE)
            .service_as_consumer(api_flow_control_id)
            .relay_as_consumer(api_flow_control_id)
            .prefix("forward_to_");

        let options = if let Some(authority) = &self.project_authority {
            let policy_access_control = self
                .policy_access_control(
                    self.project_authority.clone(),
                    Resource::new(DefaultAddress::RELAY_SERVICE, ResourceType::Relay),
                    Action::HandleMessage,
                    None,
                )
                .await?;

            let sc_flow_id = secure_channel_listener.flow_control_id();
            options
                .service_as_consumer(sc_flow_id)
                .relay_as_consumer(sc_flow_id)
                .with_service_incoming_access_control(Arc::new(
                    policy_access_control.create_incoming(),
                ))
                .authority(
                    authority.clone(),
                    self.secure_channels.identities().identities_attributes(),
                )
        } else {
            options
        };

        RelayService::create(ctx, DefaultAddress::RELAY_SERVICE, options).await?;

        Ok(())
    }

    async fn initialize_services(
        &mut self,
        ctx: &Context,
        start_default_services: bool,
    ) -> ockam_core::Result<()> {
        let api_flow_control_id = self.api_transport_flow_control_id.clone();

        if start_default_services {
            self.initialize_default_services(ctx, &api_flow_control_id)
                .await?;
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveliness checks.
        ctx.flow_controls()
            .add_consumer(DefaultAddress::ECHO_SERVICE, &api_flow_control_id);
        self.start_echoer_service(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        Ok(())
    }

    pub async fn make_connection(
        &self,
        ctx: Arc<Context>,
        addr: &MultiAddr,
        identifier: Identifier,
        authorized: Option<Identifier>,
        timeout: Option<Duration>,
    ) -> ockam_core::Result<Connection> {
        let authorized = authorized.map(|authorized| vec![authorized]);
        self.connect(ctx, addr, identifier, authorized, timeout)
            .await
    }

    /// Resolve project ID (if any), create secure channel (if needed) and create a tcp connection
    /// Returns [`Connection`]
    async fn connect(
        &self,
        ctx: Arc<Context>,
        addr: &MultiAddr,
        identifier: Identifier,
        authorized: Option<Vec<Identifier>>,
        timeout: Option<Duration>,
    ) -> ockam_core::Result<Connection> {
        debug!(?timeout, "connecting to {}", &addr);
        let connection = ConnectionBuilder::new(addr.clone())
            .instantiate(
                ctx.clone(),
                self,
                ProjectInstantiator::new(identifier.clone(), timeout),
            )
            .await?
            .instantiate(ctx.clone(), self, PlainTcpInstantiator::new())
            .await?
            .instantiate(
                ctx.clone(),
                self,
                SecureChannelInstantiator::new(&identifier, timeout, authorized),
            )
            .await?
            .build();
        connection.add_default_consumers(ctx);

        debug!("connected to {connection:?}");
        Ok(connection)
    }

    pub(crate) async fn resolve_project(
        &self,
        name: &str,
    ) -> ockam_core::Result<(MultiAddr, Identifier)> {
        let project = self.cli_state.projects().get_project_by_name(name).await?;
        Ok((
            project.project_multiaddr()?.clone(),
            project.project_identifier()?,
        ))
    }

    pub fn identifier(&self) -> Identifier {
        self.node_identifier.clone()
    }

    pub(crate) async fn get_identifier_by_name(
        &self,
        identity_name: Option<String>,
    ) -> ockam_core::Result<Identifier> {
        if let Some(name) = identity_name {
            Ok(self.cli_state.get_identifier_by_name(name.as_ref()).await?)
        } else {
            Ok(self.identifier())
        }
    }

    pub fn credential_retriever_creators(&self) -> CredentialRetrieverCreators {
        self.credential_retriever_creators.clone()
    }

    pub fn project_authority(&self) -> Option<Identifier> {
        self.project_authority.clone()
    }

    pub fn node_name(&self) -> String {
        self.node_name.clone()
    }

    pub fn tcp_transport(&self) -> &TcpTransport {
        &self.tcp_transport
    }

    pub async fn list_outlets(&self) -> Vec<OutletStatus> {
        self.registry
            .outlets
            .entries()
            .await
            .iter()
            .map(|(_, info)| OutletStatus::new(info.socket_addr, info.worker_addr.clone(), None))
            .collect()
    }

    /// Delete the current node data
    pub async fn delete_node(&self) -> ockam_core::Result<()> {
        self.cli_state.remove_node(&self.node_name).await?;
        Ok(())
    }

    pub async fn create_authority_client(
        &self,
        project: &Project,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNodeClient> {
        let caller_identifier = self
            .get_identifier_by_name(caller_identity_name)
            .await
            .into_diagnostic()?;

        let is_project_admin = self
            .cli_state
            .is_project_admin(&caller_identifier, project)
            .await
            .into_diagnostic()?;

        let credential_retriever_creator = if is_project_admin {
            self.credential_retriever_creators.project_admin.clone()
        } else {
            None
        };

        self.make_authority_node_client(
            &project.authority_identifier().into_diagnostic()?,
            project.authority_multiaddr().into_diagnostic()?,
            &caller_identifier,
            credential_retriever_creator,
        )
        .await
        .into_diagnostic()
    }

    pub async fn create_project_client(
        &self,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identity_name: Option<String>,
        credentials_enabled: CredentialsEnabled,
    ) -> miette::Result<ProjectNodeClient> {
        self.make_project_node_client(
            project_identifier,
            project_multiaddr,
            &self
                .get_identifier_by_name(caller_identity_name)
                .await
                .into_diagnostic()?,
            credentials_enabled,
        )
        .await
        .into_diagnostic()
    }

    pub(super) async fn access_control(
        &self,
        ctx: &Context,
        authority: Option<Identifier>,
        resource: Resource,
        action: Action,
        expression: Option<PolicyExpression>,
    ) -> ockam_core::Result<(
        Arc<dyn IncomingAccessControl>,
        Arc<dyn OutgoingAccessControl>,
    )> {
        let resource_name_str = resource.resource_name.as_str();
        let resource_type_str = resource.resource_type.to_string();
        let action_str = action.as_ref();
        if authority.is_some() || expression.is_some() {
            let policy_access_control = self
                .policy_access_control(authority, resource, action, expression)
                .await?;

            let incoming_ac = policy_access_control.create_incoming();
            let outgoing_ac = policy_access_control.create_outgoing(ctx).await?;

            cfg_if::cfg_if! {
                if #[cfg(feature = "std")] {
                    let incoming_ac = CachedIncomingAccessControl::new(Box::new(incoming_ac));
                    let outgoing_ac = CachedOutgoingAccessControl::new(Box::new(outgoing_ac));

                    Ok((Arc::new(incoming_ac), Arc::new(outgoing_ac)))
                } else {
                    Ok((Arc::new(incoming_ac), Arc::new(outgoing_ac)))
                }
            }
        } else {
            // If no expression is given, assume it's AllowAll, but only if no authority
            // was set neither. Why: not sure, but to behave as it was previously if there
            // is an authority set.  If there is no authority, but still some expression,
            // we use the provided policy expression
            warn! {
                resource_name = resource_name_str,
                resource_type = resource_type_str,
                action = action_str,
                "no policy access control set"
            }
            Ok((Arc::new(AllowAll), Arc::new(AllowAll)))
        }
    }

    pub fn policies(&self) -> Policies {
        self.cli_state.policies(&self.node_name)
    }

    pub fn resources(&self) -> Resources {
        self.cli_state.resources(&self.node_name)
    }

    pub async fn policy_access_control(
        &self,
        authority: Option<Identifier>,
        resource: Resource,
        action: Action,
        expression: Option<PolicyExpression>,
    ) -> ockam_core::Result<PolicyAccessControl> {
        let resource_name_str = resource.resource_name.as_str();
        let action_str = action.as_ref();

        // Populate environment with known attributes:
        let mut env = Env::new();
        env.put("resource.id", str(resource_name_str));
        env.put("action.id", str(action_str));

        // Store policy for the given resource and action
        let policies = self.policies();
        if let Some(expression) = expression {
            policies
                .store_policy_for_resource_name(
                    &resource.resource_name,
                    &action,
                    &expression.into(),
                )
                .await?;
        }
        self.resources().store_resource(&resource).await?;

        // Create the policy access control
        Ok(policies.make_policy_access_control(
            self.cli_state.identities_attributes(&self.node_name),
            resource,
            action,
            env,
            authority,
        ))
    }
}

#[derive(Debug)]
pub struct NodeManagerGeneralOptions {
    pub(super) cli_state: CliState,
    pub(super) node_name: String,
    pub(super) start_default_services: bool,
    pub(super) http_server_port: Option<u16>,
    pub(super) persistent: bool,
}

impl NodeManagerGeneralOptions {
    pub fn new(
        cli_state: CliState,
        node_name: String,
        start_default_services: bool,
        http_server_port: Option<u16>,
        persistent: bool,
    ) -> Self {
        Self {
            cli_state,
            node_name,
            start_default_services,
            http_server_port,
            persistent,
        }
    }
}

#[derive(Clone)]
/// Transport to build connection
pub struct ApiTransport {
    /// Type of transport being requested
    pub tt: TransportType,
    /// Mode of transport being requested
    pub tm: TransportMode,
    /// Socket address
    pub socket_address: SocketAddr,
    /// Worker address
    pub worker_address: String,
    /// Processor address
    pub processor_address: String,
    /// FlowControlId
    pub flow_control_id: FlowControlId,
}

#[derive(Debug)]
pub struct NodeManagerTransportOptions {
    api_transport_flow_control_id: FlowControlId,
    tcp_transport: TcpTransport,
}

impl NodeManagerTransportOptions {
    pub fn new(api_transport_flow_control_id: FlowControlId, tcp_transport: TcpTransport) -> Self {
        Self {
            api_transport_flow_control_id,
            tcp_transport,
        }
    }
}
