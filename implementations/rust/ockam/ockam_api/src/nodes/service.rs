//! Node Manager (Node Man, the superhero that we deserve)

use std::collections::BTreeMap;
use std::error::Error as _;
use std::net::SocketAddr;
use std::path::PathBuf;

use minicbor::Decoder;

pub use node_identities::*;
use ockam::identity::{
    Credentials, CredentialsServer, CredentialsServerModule, Identities, IdentitiesRepository,
    IdentitiesVault, IdentityAttributesReader, IdentityAttributesWriter,
};
use ockam::identity::{IdentityIdentifier, SecureChannels};
use ockam::{
    Address, Context, ForwardingService, ForwardingServiceOptions, Result, Routed, TcpTransport,
    Worker,
};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, PolicyStorage, Resource};
use ockam_core::api::{Error, Method, Request, Response, ResponseBuilder, Status};
use ockam_core::compat::{
    boxed::Box,
    string::String,
    sync::{Arc, Mutex},
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy};
use ockam_core::IncomingAccessControl;
use ockam_core::{AllowAll, AsyncTryClone};
use ockam_identity::TrustContext;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::tokio;
use ockam_node::tokio::task::JoinHandle;

use crate::bootstrapped_identities_store::BootstrapedIdentityStore;
use crate::bootstrapped_identities_store::PreTrustedIdentities;
use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::config::cli::TrustContextConfig;
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::nodes::connection::{
    Connection, ConnectionInstance, ConnectionInstanceBuilder, PlainTcpInstantiator,
    ProjectInstantiator, SecureChannelInstantiator,
};
use crate::nodes::models::base::NodeStatus;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::nodes::registry::KafkaServiceKind;
use crate::session::sessions::Sessions;
use crate::session::Medic;
use crate::DefaultAddress;
use crate::RpcProxyService;

use super::registry::Registry;

mod credentials;
mod flow_controls;
mod forwarder;
pub mod message;
mod node_identities;
mod node_services;
mod policy;
mod portals;
mod secure_channel;
mod transport;

const TARGET: &str = "ockam_api::nodemanager::service";

pub(crate) type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

// TODO: Move to multiaddr implementation
pub(crate) fn invalid_multiaddr_error() -> ockam_core::Error {
    ockam_core::Error::new(Origin::Core, Kind::Invalid, "Invalid multiaddr")
}

// TODO: Move to multiaddr implementation
pub(crate) fn map_multiaddr_err(_err: ockam_multiaddr::Error) -> ockam_core::Error {
    invalid_multiaddr_error()
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeManager {
    pub(crate) cli_state: CliState,
    node_name: String,
    api_transport: ApiTransport,
    pub(crate) tcp_transport: TcpTransport,
    pub(crate) controller_identity_id: IdentityIdentifier,
    skip_defaults: bool,
    enable_credential_checks: bool,
    identifier: IdentityIdentifier,
    pub(crate) secure_channels: Arc<SecureChannels>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    trust_context: Option<TrustContext>,
    pub(crate) registry: Registry,
    sessions: Arc<Mutex<Sessions>>,
    medic: JoinHandle<Result<(), ockam_core::Error>>,
    policies: Arc<dyn PolicyStorage>,
}

impl NodeManager {
    pub(super) fn identifier(&self) -> IdentityIdentifier {
        self.identifier.clone()
    }

    pub(super) fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities().repository().clone()
    }

    pub(super) fn attributes_writer(&self) -> Arc<dyn IdentityAttributesWriter> {
        self.identities_repository().as_attributes_writer()
    }

    pub(super) fn attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        self.identities_repository().as_attributes_reader()
    }

    pub(super) fn credentials(&self) -> Arc<dyn Credentials> {
        self.identities().credentials()
    }

    pub(super) fn credentials_service(&self) -> Arc<dyn CredentialsServer> {
        Arc::new(CredentialsServerModule::new(self.credentials()))
    }

    pub(super) fn secure_channels_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.secure_channels.vault().clone()
    }
}

pub struct NodeManagerWorker {
    node_manager: Arc<RwLock<NodeManager>>,
}

impl NodeManagerWorker {
    pub fn new(node_manager: NodeManager) -> Self {
        NodeManagerWorker {
            node_manager: Arc::new(RwLock::new(node_manager)),
        }
    }

    pub fn get(&mut self) -> &mut Arc<RwLock<NodeManager>> {
        &mut self.node_manager
    }
}

pub struct IdentityOverride {
    pub identity: Vec<u8>,
    pub vault_path: PathBuf,
}

impl NodeManager {
    async fn access_control(
        &self,
        r: &Resource,
        a: &Action,
        trust_context_id: Option<&str>,
        custom_default: Option<&Expr>,
    ) -> Result<Arc<dyn IncomingAccessControl>> {
        if let Some(tcid) = trust_context_id {
            // Populate environment with known attributes:
            let mut env = Env::new();
            env.put("resource.id", str(r.as_str()));
            env.put("action.id", str(a.as_str()));
            env.put("resource.project_id", str(tcid.to_string()));
            env.put("resource.trust_context_id", str(tcid));

            // Check if a policy exists for (resource, action) and if not, then
            // create or use a default entry:
            if self.policies.get_policy(r, a).await?.is_none() {
                let fallback = match custom_default {
                    Some(e) => e.clone(),
                    None => and([
                        eq([ident("resource.project_id"), ident("subject.project_id")]), // TODO: DEPRECATE - Removing PROJECT_ID attribute in favor of TRUST_CONTEXT_ID
                                                                                         /*
                                                                                         * TODO: replace the project_id check for trust_context_id.  For now the
                                                                                         * existing authority deployed doesn't know about trust_context so this is to
                                                                                         * be done after updating deployed authorities.
                                                                                         eq([
                                                                                             ident("resource.trust_context_id"),
                                                                                             ident("subject.trust_context_id"),
                                                                                         ]),
                                                                                         */
                    ]),
                };
                self.policies.set_policy(r, a, &fallback).await?
            }
            let policies = self.policies.clone();
            Ok(Arc::new(PolicyAccessControl::new(
                policies,
                self.identities_repository(),
                r.clone(),
                a.clone(),
                env,
            )))
        } else {
            Ok(Arc::new(AllowAll))
        }
    }

    pub(crate) fn trust_context(&self) -> Result<&TrustContext> {
        self.trust_context
            .as_ref()
            .ok_or_else(|| ApiError::generic("Trust context doesn't exist"))
    }
}

pub struct NodeManagerGeneralOptions {
    cli_state: CliState,
    node_name: String,
    skip_defaults: bool,
    pre_trusted_identities: Option<PreTrustedIdentities>,
}

impl NodeManagerGeneralOptions {
    pub fn new(
        cli_state: CliState,
        node_name: String,
        skip_defaults: bool,
        pre_trusted_identities: Option<PreTrustedIdentities>,
    ) -> Self {
        Self {
            cli_state,
            node_name,
            skip_defaults,
            pre_trusted_identities,
        }
    }
}

pub struct NodeManagerProjectsOptions {
    projects: BTreeMap<String, ProjectLookup>,
}

impl NodeManagerProjectsOptions {
    pub fn new(projects: BTreeMap<String, ProjectLookup>) -> Self {
        Self { projects }
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

pub struct NodeManagerTransportOptions {
    api_transport: ApiTransport,
    tcp_transport: TcpTransport,
}

impl NodeManagerTransportOptions {
    pub fn new(api_transport: ApiTransport, tcp_transport: TcpTransport) -> Self {
        Self {
            api_transport,
            tcp_transport,
        }
    }
}

pub struct NodeManagerTrustOptions {
    trust_context_config: Option<TrustContextConfig>,
}

impl NodeManagerTrustOptions {
    pub fn new(trust_context_config: Option<TrustContextConfig>) -> Self {
        Self {
            trust_context_config,
        }
    }
}

impl NodeManager {
    /// Create a new NodeManager with the node name from the ockam CLI
    pub async fn create(
        ctx: &Context,
        general_options: NodeManagerGeneralOptions,
        projects_options: NodeManagerProjectsOptions,
        transport_options: NodeManagerTransportOptions,
        trust_options: NodeManagerTrustOptions,
    ) -> Result<Self> {
        let api_transport_id = random_alias();
        let mut transports = BTreeMap::new();
        transports.insert(
            api_transport_id.clone(),
            transport_options.api_transport.clone(),
        );

        let cli_state = general_options.cli_state;
        let node_state = cli_state.nodes.get(&general_options.node_name)?;

        let repository: Arc<dyn IdentitiesRepository> =
            cli_state.identities.identities_repository().await?;

        //TODO: fix this.  Either don't require it to be a bootstrappedidentitystore (and use the
        //trait instead),  or pass it from the general_options always.
        let vault: Arc<dyn IdentitiesVault> = node_state.config().vault().await?;
        let identities_repository: Arc<dyn IdentitiesRepository> =
            Arc::new(match general_options.pre_trusted_identities {
                None => BootstrapedIdentityStore::new(
                    Arc::new(PreTrustedIdentities::new_from_string("{}")?),
                    repository.clone(),
                ),
                Some(f) => BootstrapedIdentityStore::new(Arc::new(f), repository.clone()),
            });

        let secure_channels = SecureChannels::builder()
            .with_identities_vault(vault)
            .with_identities_repository(identities_repository.clone())
            .build();

        let policies: Arc<dyn PolicyStorage> = Arc::new(node_state.policies_storage().await?);

        let medic = Medic::new();
        let sessions = medic.sessions();

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            api_transport: transport_options.api_transport,
            tcp_transport: transport_options.tcp_transport,
            controller_identity_id: Self::load_controller_identity_id()?,
            skip_defaults: general_options.skip_defaults,
            enable_credential_checks: trust_options.trust_context_config.is_some()
                && trust_options
                    .trust_context_config
                    .as_ref()
                    .unwrap()
                    .authority()
                    .is_ok(),
            identifier: node_state.config().identifier().await?,
            secure_channels,
            projects: Arc::new(projects_options.projects),
            trust_context: None,
            registry: Default::default(),
            medic: {
                let ctx = ctx.async_try_clone().await?;
                tokio::spawn(medic.start(ctx))
            },
            sessions,
            policies,
        };

        info!("NodeManager::create: {}", s.node_name);
        if !general_options.skip_defaults {
            info!("NodeManager::create: starting default services");
            if let Some(tc) = trust_options.trust_context_config {
                info!("NodeManager::create: configuring trust context");
                s.configure_trust_context(&tc).await?;
            }
        }

        Ok(s)
    }

    async fn configure_trust_context(&mut self, tc: &TrustContextConfig) -> Result<()> {
        self.trust_context = Some(
            tc.to_trust_context(
                self.secure_channels.clone(),
                Some(self.tcp_transport.async_try_clone().await?),
            )
            .await?,
        );

        info!("NodeManager::configure_trust_context: trust context configured");

        Ok(())
    }

    async fn initialize_defaults(
        &mut self,
        ctx: &Context,
        api_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        // Start services
        ctx.flow_controls().add_consumer(
            DefaultAddress::UPPERCASE_SERVICE,
            api_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );
        self.start_uppercase_service_impl(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
            .await?;

        ForwardingService::create(
            ctx,
            DefaultAddress::FORWARDING_SERVICE,
            ForwardingServiceOptions::new()
                .service_as_consumer(
                    api_flow_control_id,
                    FlowControlPolicy::SpawnerAllowMultipleMessages,
                )
                .forwarder_as_consumer(
                    api_flow_control_id,
                    FlowControlPolicy::SpawnerAllowMultipleMessages,
                ),
        )
        .await?;

        self.create_secure_channel_listener_impl(
            DefaultAddress::SECURE_CHANNEL_LISTENER.into(),
            None, // Not checking identifiers here in favor of credential check
            None,
            None,
            ctx,
        )
        .await?;

        // If we've been configured with a trust context, we can start Credential Exchange service
        if let Ok(tc) = self.trust_context() {
            self.start_credentials_service_impl(
                ctx,
                tc.clone(),
                DefaultAddress::CREDENTIALS_SERVICE.into(),
                false,
            )
            .await?;
        }

        Ok(())
    }

    /// Resolve project ID (if any), create secure channel (if needed) and create a tcp connection
    /// Returns [`ConnectionInstance`]
    pub(crate) async fn connect(
        node_manager: Arc<RwLock<NodeManager>>,
        connection: Connection<'_>,
    ) -> Result<ConnectionInstance> {
        debug!("connecting to {}", &connection.addr);
        let context = Arc::new(connection.ctx.async_try_clone().await?);

        let tcp_transport = node_manager
            .clone()
            .read()
            .await
            .tcp_transport
            .async_try_clone()
            .await?;

        let connection_instance = ConnectionInstanceBuilder::new(connection.addr.clone())
            .instantiate(ProjectInstantiator::new(
                context.clone(),
                node_manager.clone(),
                connection.timeout,
                connection.credential_name.map(|x| x.to_string()),
                connection.identity_name.map(|x| x.to_string()),
            ))
            .await?
            .instantiate(PlainTcpInstantiator::new(tcp_transport))
            .await?
            .instantiate(SecureChannelInstantiator::new(
                context.clone(),
                node_manager.clone(),
                connection.timeout,
                connection.authorized_identities,
            ))
            .await?
            .build();

        debug!("connected to {connection_instance:?}");

        if connection.add_default_consumers {
            connection_instance
                .add_consumer(&context, &DefaultAddress::SECURE_CHANNEL_LISTENER.into());
            connection_instance.add_consumer(&context, &DefaultAddress::UPPERCASE_SERVICE.into());
            connection_instance.add_consumer(&context, &DefaultAddress::ECHO_SERVICE.into());
        }

        Ok(connection_instance)
    }

    pub(crate) fn resolve_project(&self, name: &str) -> Result<(MultiAddr, IdentityIdentifier)> {
        if let Some(info) = self.projects.get(name) {
            let node_route = info
                .node_route
                .as_ref()
                .ok_or_else(|| ApiError::generic("Project should have node route set"))?
                .clone();
            let identity_id = info
                .identity_id
                .as_ref()
                .ok_or_else(|| ApiError::generic("Project should have identity set"))?
                .clone();
            Ok((node_route, identity_id))
        } else {
            Err(ApiError::message(format!("project {name} not found")))
        }
    }
}

impl NodeManagerWorker {
    //////// Request matching and response handling ////////

    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        debug! {
            target: TARGET,
            id     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            body   = %req.has_body(),
            "request"
        }

        use Method::*;
        let path = req.path();
        let path_segments = req.path_segments::<5>();
        let method = match req.method() {
            Some(m) => m,
            None => todo!(),
        };

        let r = match (method, path_segments.as_slice()) {
            // ==*== Basic node information ==*==
            // TODO: create, delete, destroy remote nodes
            (Get, ["node"]) => {
                let node_manager = self.node_manager.read().await;
                Response::ok(req.id())
                    .body(NodeStatus::new(
                        &node_manager.node_name,
                        "Running",
                        ctx.list_workers().await?.len() as u32,
                        std::process::id() as i32,
                    ))
                    .to_vec()?
            }

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_connections(req, &node_manager.tcp_transport)
                    .to_vec()?
            }
            (Get, ["node", "tcp", "connection", address]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_connection(req, &node_manager.tcp_transport, address.to_string())
                    .to_vec()?
            }
            (Post, ["node", "tcp", "connection"]) => {
                self.create_tcp_connection(req, dec, ctx).await?.to_vec()?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                self.delete_tcp_connection(req, dec).await?.to_vec()?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_listeners(req, &node_manager.tcp_transport)
                    .to_vec()?
            }
            (Get, ["node", "tcp", "listener", address]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_listener(req, &node_manager.tcp_transport, address.to_string())
                    .to_vec()?
            }
            (Post, ["node", "tcp", "listener"]) => {
                self.create_tcp_listener(req, dec).await?.to_vec()?
            }
            (Delete, ["node", "tcp", "listener"]) => {
                self.delete_tcp_listener(req, dec).await?.to_vec()?
            }

            // ==*== Credential ==*==
            (Post, ["node", "credentials", "actions", "get"]) => self
                .get_credential(req, dec, ctx)
                .await?
                .either(ResponseBuilder::to_vec, ResponseBuilder::to_vec)?,
            (Post, ["node", "credentials", "actions", "present"]) => {
                self.present_credential(req, dec, ctx).await?.to_vec()?
            }

            // ==*== Secure channels ==*==
            // TODO: Change to RequestBuilder format
            (Get, ["node", "secure_channel"]) => {
                let node_manager = self.node_manager.read().await;
                self.list_secure_channels(req, &node_manager.registry)
                    .to_vec()?
            }
            (Get, ["node", "secure_channel_listener"]) => {
                let node_manager = self.node_manager.read().await;
                self.list_secure_channel_listener(req, &node_manager.registry)
                    .to_vec()?
            }
            (Post, ["node", "secure_channel"]) => {
                self.create_secure_channel(req, dec, ctx).await?.to_vec()?
            }
            (Delete, ["node", "secure_channel"]) => {
                self.delete_secure_channel(req, dec, ctx).await?.to_vec()?
            }
            (Get, ["node", "show_secure_channel"]) => {
                self.show_secure_channel(req, dec).await?.to_vec()?
            }
            (Post, ["node", "secure_channel_listener"]) => self
                .create_secure_channel_listener(req, dec, ctx)
                .await?
                .to_vec()?,
            (Delete, ["node", "secure_channel_listener"]) => self
                .delete_secure_channel_listener(req, dec)
                .await?
                .to_vec()?,
            (Get, ["node", "show_secure_channel_listener"]) => {
                self.show_secure_channel_listener(req, dec).await?
            }

            // ==*== Services ==*==
            (Post, ["node", "services", DefaultAddress::IDENTITY_SERVICE]) => {
                self.start_identity_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", DefaultAddress::AUTHENTICATED_SERVICE]) => self
                .start_authenticated_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => self
                .start_uppercase_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => {
                self.start_echoer_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                self.start_hop_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", DefaultAddress::DIRECT_AUTHENTICATOR]) => self
                .start_authenticator_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::VERIFIER]) => {
                self.start_verifier_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", DefaultAddress::CREDENTIALS_SERVICE]) => self
                .start_credentials_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::OKTA_IDENTITY_PROVIDER]) => self
                .start_okta_identity_provider_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => {
                self.start_kafka_consumer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => self
                .delete_kafka_service(ctx, req, dec, KafkaServiceKind::Consumer)
                .await?
                .to_vec()?,
            (Post, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => {
                self.start_kafka_producer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => self
                .delete_kafka_service(ctx, req, dec, KafkaServiceKind::Producer)
                .await?
                .to_vec()?,
            (Get, ["node", "services"]) => self.list_services(req).await?,
            (Get, ["node", "services", service_type]) => {
                self.list_services_of_type(req, service_type).await?
            }

            // ==*== Forwarder commands ==*==
            (Get, ["node", "forwarder", remote_address]) => {
                self.show_forwarder(req, remote_address).await?.to_vec()?
            }
            (Get, ["node", "forwarder"]) => {
                let forwarder_registry = {
                    let node_manager = self.node_manager.read().await;
                    &node_manager.registry.forwarders.clone()
                };
                self.get_forwarders(req, forwarder_registry)
                    .await
                    .to_vec()?
            }
            (Delete, ["node", "forwarder", remote_address]) => self
                .delete_forwarder(ctx, req, remote_address)
                .await?
                .to_vec()?,
            (Post, ["node", "forwarder"]) => self.create_forwarder(ctx, req.id(), dec).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => {
                let inlet_registry = {
                    let node_manager = self.node_manager.read().await;
                    &node_manager.registry.inlets.clone()
                };
                self.get_inlets(req, inlet_registry).to_vec()?
            }
            (Get, ["node", "inlet", alias]) => self.show_inlet(req, alias).await?.to_vec()?,
            (Get, ["node", "outlet"]) => {
                let outlet_registry = {
                    let node_manager = self.node_manager.read().await;
                    &node_manager.registry.outlets.clone()
                };
                self.get_outlets(req, outlet_registry).to_vec()?
            }
            (Get, ["node", "outlet", alias]) => self.show_outlet(req, alias).await?.to_vec()?,
            (Post, ["node", "inlet"]) => self.create_inlet(req, dec, ctx).await?.to_vec()?,
            (Post, ["node", "outlet"]) => self.create_outlet(ctx, req, dec).await?.to_vec()?,
            (Delete, ["node", "outlet", alias]) => {
                self.delete_outlet(req, alias).await?.to_vec()?
            }
            (Delete, ["node", "inlet", alias]) => self.delete_inlet(req, alias).await?.to_vec()?,
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                self.add_consumer(ctx, req, dec)?.to_vec()?
            }

            // ==*== Workers ==*==
            (Get, ["node", "workers"]) => {
                let workers = ctx.list_workers().await?;

                let mut list = Vec::new();
                workers
                    .iter()
                    .for_each(|addr| list.push(WorkerStatus::new(addr.address())));

                Response::ok(req.id())
                    .body(WorkerList::new(list))
                    .to_vec()?
            }

            (Post, ["policy", resource, action]) => self
                .node_manager
                .read()
                .await
                .add_policy(resource, action, req, dec)
                .await?
                .to_vec()?,
            (Get, ["policy", resource]) => self
                .node_manager
                .read()
                .await
                .list_policies(req, resource)
                .await?
                .to_vec()?,
            (Get, ["policy", resource, action]) => self
                .node_manager
                .read()
                .await
                .get_policy(req, resource, action)
                .await?
                .either(ResponseBuilder::to_vec, ResponseBuilder::to_vec)?,
            (Delete, ["policy", resource, action]) => self
                .node_manager
                .read()
                .await
                .del_policy(req, resource, action)
                .await?
                .to_vec()?,

            // ==*== Spaces ==*==
            (Post, ["v0", "spaces"]) => self.create_space(ctx, dec).await?,
            (Get, ["v0", "spaces"]) => self.list_spaces(ctx, dec).await?,
            (Get, ["v0", "spaces", id]) => self.get_space(ctx, dec, id).await?,
            (Delete, ["v0", "spaces", id]) => self.delete_space(ctx, dec, id).await?,

            // ==*== Projects ==*==
            (Post, ["v0", "projects", space_id]) => self.create_project(ctx, dec, space_id).await?,
            (Get, ["v0", "projects"]) => self.list_projects(ctx, dec).await?,
            (Get, ["v0", "projects", project_id]) => self.get_project(ctx, dec, project_id).await?,
            (Delete, ["v0", "projects", space_id, project_id]) => {
                self.delete_project(ctx, dec, space_id, project_id).await?
            }

            // ==*== Enroll ==*==
            (Post, ["v0", "enroll", "auth0"]) => self.enroll_auth0(ctx, dec).await?,
            (Get, ["v0", "enroll", "token"]) => self.generate_enrollment_token(ctx, dec).await?,
            (Put, ["v0", "enroll", "token"]) => {
                self.authenticate_enrollment_token(ctx, dec).await?
            }

            // ==*== Subscriptions ==*==
            (Post, ["subscription"]) => self.activate_subscription(ctx, dec).await?,
            (Get, ["subscription", id]) => self.get_subscription(ctx, dec, id).await?,
            (Get, ["subscription"]) => self.list_subscriptions(ctx, dec).await?,
            (Put, ["subscription", id, "contact_info"]) => {
                self.update_subscription_contact_info(ctx, dec, id).await?
            }
            (Put, ["subscription", id, "space_id"]) => {
                self.update_subscription_space(ctx, dec, id).await?
            }
            (Put, ["subscription", id, "unsubscribe"]) => self.unsubscribe(ctx, dec, id).await?,

            // ==*== Addons ==*==
            (Get, [project_id, "addons"]) => self.list_addons(ctx, dec, project_id).await?,
            (Put, [project_id, "addons", addon_id]) => {
                self.configure_addon(ctx, dec, project_id, addon_id).await?
            }
            (Delete, [project_id, "addons", addon_id]) => {
                self.disable_addon(ctx, dec, project_id, addon_id).await?
            }

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => self.send_message(ctx, req, dec).await?,

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req.id())
                    .body(format!("Invalid endpoint: {path}"))
                    .to_vec()?
            }
        };
        Ok(r)
    }
}

#[ockam::worker]
impl Worker for NodeManagerWorker {
    type Message = Vec<u8>;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        let mut node_manager = self.node_manager.write().await;
        let api_flow_control_id = node_manager.api_transport.flow_control_id.clone();

        if !node_manager.skip_defaults {
            node_manager
                .initialize_defaults(ctx, &api_flow_control_id)
                .await?;
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveness checks.
        ctx.flow_controls().add_consumer(
            DefaultAddress::ECHO_SERVICE,
            &api_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );
        node_manager
            .start_echoer_service_impl(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        ctx.flow_controls().add_consumer(
            DefaultAddress::RPC_PROXY,
            &api_flow_control_id,
            FlowControlPolicy::SpawnerAllowMultipleMessages,
        );
        ctx.start_worker(
            DefaultAddress::RPC_PROXY,
            RpcProxyService::new(),
            AllowAll,
            AllowAll,
        )
        .await?;

        Ok(())
    }

    async fn shutdown(&mut self, _: &mut Self::Context) -> Result<()> {
        let node_manager = self.node_manager.read().await;
        node_manager.medic.abort();
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let mut dec = Decoder::new(msg.as_body());
        let req: Request = match dec.decode() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to decode request: {:?}", e);
                return Ok(());
            }
        };

        let r = match self.handle_request(ctx, &req, &mut dec).await {
            Ok(r) => r,
            Err(err) => {
                error! {
                    target: TARGET,
                    re     = %req.id(),
                    method = ?req.method(),
                    path   = %req.path(),
                    code   = %err.code(),
                    cause  = ?err.source(),
                    "failed to handle request"
                }
                let err = Error::new(req.path())
                    .with_message(format!("failed to handle request: {err} {req:?}"));
                Response::builder(req.id(), Status::InternalServerError)
                    .body(err)
                    .to_vec()?
            }
        };
        debug! {
            target: TARGET,
            re     = %req.id(),
            method = ?req.method(),
            path   = %req.path(),
            "responding"
        }
        ctx.send(msg.return_route(), r).await
    }
}
