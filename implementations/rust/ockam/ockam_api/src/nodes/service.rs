//! Node Manager (Node Man, the superhero that we deserve)

use minicbor::Decoder;
use ockam::identity::{
    Credentials, CredentialsServer, CredentialsServerModule, Identities,
    IdentitiesRepository, IdentitiesVault, IdentityAttributesReader, IdentityAttributesWriter,
};
use ockam::identity::{Identity, IdentityIdentifier, SecureChannels};
use ockam::{Address, Context, ForwardingService, Result, Routed, TcpTransport, Worker};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, PolicyStorage, Resource};
use ockam_core::api::{Error, Method, Request, Response, ResponseBuilder, Status};
use ockam_core::compat::{
    boxed::Box,
    string::String,
    sync::{Arc, Mutex},
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{route, AllowAll, AsyncTryClone, IncomingAccessControl};
use ockam_multiaddr::proto::{Project, Secure};
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::tokio;
use ockam_node::tokio::task::JoinHandle;
use std::collections::BTreeMap;
use std::error::Error as _;
use std::net::SocketAddr;
use std::path::PathBuf;

use super::models::secure_channel::CredentialExchangeMode;
use super::registry::Registry;
use crate::bootstrapped_identities_store::BootstrapedIdentityStore;
use crate::bootstrapped_identities_store::PreTrustedIdentities;
use crate::cli_state::CliState;
use crate::config::cli::TrustContextConfig;
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::nodes::connection::Connection;
use crate::nodes::models::base::NodeStatus;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::rpc_proxy::RpcProxyService;
use crate::session::util::{starts_with_host_tcp, starts_with_secure};
use crate::session::{Medic, Sessions};
use crate::{
    local_multiaddr_to_route, multiaddr_to_route, route_to_multiaddr, try_address_to_multiaddr,
    DefaultAddress,
};

pub mod message;

mod credentials;
mod forwarder;
mod node_identities;
mod node_services;
mod policy;
mod portals;
mod secure_channel;
mod transport;

pub use node_identities::*;
use ockam_identity::TrustContext;

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

type Transports = BTreeMap<Alias, ApiTransport>;

/// Node manager provides a messaging API to interact with the current node
pub struct NodeManager {
    pub(crate) cli_state: CliState,
    node_name: String,
    transports: Transports,
    pub(crate) tcp_transport: TcpTransport,
    pub(crate) controller_identity_id: IdentityIdentifier,
    skip_defaults: bool,
    enable_credential_checks: bool,
    identity: Identity,
    pub(crate) secure_channels: Arc<SecureChannels>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    trust_context: Option<TrustContext>,
    pub(crate) registry: Registry,
    sessions: Arc<Mutex<Sessions>>,
    medic: JoinHandle<Result<(), ockam_core::Error>>,
    policies: Arc<dyn PolicyStorage>,
    pub(crate) flow_controls: FlowControls,
}

impl NodeManager {
    pub(super) fn identity(&self) -> Identity {
        self.identity.clone()
    }

    pub(super) fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    pub(super) fn identities_vault(&self) -> Arc<dyn IdentitiesVault> {
        self.identities().vault()
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
            // TODO: @ac allow passing this as a cli argument
            Ok(Arc::new(AllowAll))
        }
    }

    pub(crate) fn trust_context(&self) -> Result<&TrustContext> {
        self.trust_context
            .as_ref()
            .ok_or_else(|| ApiError::generic("Trust context doesn't exist"))
    }

    pub fn flow_controls(&self) -> &FlowControls {
        &self.flow_controls
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
    pub worker_address: Address,
    /// FlowControlId
    pub flow_control_id: Option<FlowControlId>,
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

pub(crate) struct ConnectResult {
    pub(crate) secure_channel: MultiAddr,
    pub(crate) suffix: MultiAddr,
    pub(crate) flow_control_id: Option<FlowControlId>,
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
        transports.insert(api_transport_id.clone(), transport_options.api_transport);

        let cli_state = general_options.cli_state;
        let node_state = cli_state.nodes.get(&general_options.node_name)?;

        let repository: Arc<dyn IdentitiesRepository> =
            cli_state.identities.identities_repository().await?;

        //TODO: fix this.  Either don't require it to be a bootstrappedidentitystore (and use the
        //trait instead),  or pass it from the general_options always.
        let vault: Arc<dyn IdentitiesVault> = Arc::new(node_state.config.vault().await?);
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
            .with_identities_repository(identities_repository)
            .build();

        let policies: Arc<dyn PolicyStorage> = Arc::new(node_state.policies_storage().await?);

        let identity = node_state.config.default_identity().await?;

        let flow_controls = FlowControls::default();
        let medic = Medic::new(flow_controls.clone());
        let sessions = medic.sessions();

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            transports,
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
            identity,
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
            flow_controls,
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

    async fn initialize_defaults(&mut self, ctx: &Context) -> Result<()> {
        // Start services
        self.start_vault_service_impl(ctx, DefaultAddress::VAULT_SERVICE.into())
            .await?;
        self.start_identity_service_impl(ctx, DefaultAddress::IDENTITY_SERVICE.into())
            .await?;
        self.start_authenticated_service_impl(ctx, DefaultAddress::AUTHENTICATED_SERVICE.into())
            .await?;
        self.start_uppercase_service_impl(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
            .await?;
        self.start_hop_service_impl(ctx, DefaultAddress::HOP_SERVICE.into())
            .await?;

        ForwardingService::create(
            ctx,
            DefaultAddress::FORWARDING_SERVICE,
            AllowAll, // FIXME: @ac
            AllowAll, // FIXME: @ac
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
    ///
    /// Returns the secure channel worker address (if any) and the remainder
    /// of the address argument.
    pub(crate) async fn connect(&mut self, connection: Connection<'_>) -> Result<ConnectResult> {
        let add_default_consumers = connection.add_default_consumers;

        let res = self.connect_impl(connection).await?;

        if add_default_consumers {
            if let Some(flow_control_id) = &res.flow_control_id {
                self.flow_controls.add_consumer(
                    &DefaultAddress::SECURE_CHANNEL_LISTENER.into(),
                    flow_control_id,
                    FlowControlPolicy::ProducerAllowMultiple,
                );
                self.flow_controls.add_consumer(
                    &DefaultAddress::UPPERCASE_SERVICE.into(),
                    flow_control_id,
                    FlowControlPolicy::ProducerAllowMultiple,
                );
                self.flow_controls.add_consumer(
                    &DefaultAddress::ECHO_SERVICE.into(),
                    flow_control_id,
                    FlowControlPolicy::ProducerAllowMultiple,
                );
            }
        }

        Ok(res)
    }
    pub(crate) async fn connect_impl(
        &mut self,
        connection: Connection<'_>,
    ) -> Result<ConnectResult> {
        let Connection {
            ctx,
            addr,
            identity_name,
            credential_name,
            authorized_identities,
            timeout,
            ..
        } = connection;

        let transport = &self.tcp_transport;

        if let Some(p) = addr.first() {
            if p.code() == Project::CODE {
                let p = p
                    .cast::<Project>()
                    .ok_or_else(|| ApiError::message("invalid project protocol in multiaddr"))?;
                let (a, i) = self.resolve_project(&p)?;
                debug!(addr = %a, "creating secure channel");
                let route = multiaddr_to_route(&a, transport, &self.flow_controls)
                    .await
                    .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                let i = Some(vec![i]);
                let m = CredentialExchangeMode::Oneway;
                let (sc_address, sc_flow_control_id) = self
                    .create_secure_channel_impl(
                        route.route,
                        i,
                        m,
                        timeout,
                        identity_name.map(|i| i.to_string()),
                        ctx,
                        credential_name.map(|c| c.to_string()),
                    )
                    .await?;
                let a = MultiAddr::default().try_with(addr.iter().skip(1))?;

                let res = ConnectResult {
                    secure_channel: try_address_to_multiaddr(&sc_address)?,
                    suffix: a,
                    flow_control_id: Some(sc_flow_control_id),
                };

                return Ok(res);
            }
        }

        if let Some(pos1) = starts_with_host_tcp(addr) {
            debug!(%addr, "creating a tcp connection");
            let (a1, b1) = addr.split(pos1);
            return match starts_with_secure(&b1) {
                Some(pos2) => {
                    let route = multiaddr_to_route(&a1, transport, &self.flow_controls)
                        .await
                        .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                    debug!(%addr, "creating a secure channel");
                    let (a2, b2) = b1.split(pos2);
                    let m = CredentialExchangeMode::Mutual;
                    let r2 = local_multiaddr_to_route(&a2)
                        .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                    let (sc_address, sc_flow_control_id) = self
                        .create_secure_channel_impl(
                            route![route.route, r2],
                            authorized_identities.clone(),
                            m,
                            timeout,
                            identity_name.map(|i| i.to_string()),
                            ctx,
                            credential_name.map(|c| c.to_string()),
                        )
                        .await?;

                    let res = ConnectResult {
                        secure_channel: try_address_to_multiaddr(&sc_address)?,
                        suffix: b2,
                        flow_control_id: Some(sc_flow_control_id),
                    };

                    Ok(res)
                }
                None => {
                    let route = multiaddr_to_route(addr, transport, &self.flow_controls)
                        .await
                        .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;

                    let res = ConnectResult {
                        secure_channel: route_to_multiaddr(&route.route)
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?,
                        suffix: Default::default(),
                        flow_control_id: route.flow_control_id,
                    };

                    Ok(res)
                }
            };
        }

        if Some(Secure::CODE) == addr.last().map(|p| p.code()) {
            debug!(%addr, "creating secure channel");
            let r = local_multiaddr_to_route(addr)
                .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
            let m = CredentialExchangeMode::Mutual;
            let (sc_address, sc_flow_control_id) = self
                .create_secure_channel_impl(
                    r,
                    authorized_identities.clone(),
                    m,
                    timeout,
                    None,
                    ctx,
                    None,
                )
                .await?;

            let res = ConnectResult {
                secure_channel: try_address_to_multiaddr(&sc_address)?,
                suffix: Default::default(),
                flow_control_id: Some(sc_flow_control_id),
            };

            return Ok(res);
        }

        let res = ConnectResult {
            secure_channel: Default::default(),
            suffix: addr.clone(),
            flow_control_id: None,
        };

        Ok(res)
    }

    fn resolve_project(&self, name: &str) -> Result<(MultiAddr, IdentityIdentifier)> {
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
                        node_manager.transports.len() as u32,
                    ))
                    .to_vec()?
            }

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_con_or_list(req, &node_manager.transports, TransportMode::Connect)
                    .to_vec()?
            }
            (Get, ["node", "tcp", "connection", id]) => {
                self.get_transport(req, id, TransportType::Tcp, TransportMode::Connect)
                    .await?
            }
            (Post, ["node", "tcp", "connection"]) => {
                self.add_transport(req, dec).await?.to_vec()?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                self.delete_transport(req, dec).await?.to_vec()?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_con_or_list(
                    req,
                    &node_manager.transports.clone(),
                    TransportMode::Listen,
                )
                    .to_vec()?
            }
            (Get, ["node", "tcp", "listener", id]) => {
                self.get_transport(req, id, TransportType::Tcp, TransportMode::Listen)
                    .await?
            }
            (Post, ["node", "tcp", "listener"]) => self.add_transport(req, dec).await?.to_vec()?,
            (Delete, ["node", "tcp", "listener"]) => {
                self.delete_transport(req, dec).await?.to_vec()?
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
            (Get, ["node", "show_secure_channel_listener"]) => self
                .show_secure_channel_listener(req, dec)
                .await?
                .to_vec()?,

            // ==*== Services ==*==
            (Post, ["node", "services", DefaultAddress::VAULT_SERVICE]) => {
                self.start_vault_service(ctx, req, dec).await?.to_vec()?
            }
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
            (Post, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => {
                self.start_kafka_producer_service(ctx, req, dec).await?
            }
            (Get, ["node", "services"]) => {
                let node_manager = self.node_manager.read().await;
                self.list_services(req, &node_manager.registry).to_vec()?
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
            (Post, ["node", "outlet"]) => self.create_outlet(req, dec).await?.to_vec()?,
            (Delete, ["node", "outlet", alias]) => {
                self.delete_outlet(req, alias).await?.to_vec()?
            }
            (Delete, ["node", "inlet", alias]) => self.delete_inlet(req, alias).await?.to_vec()?,
            (Delete, ["node", "portal"]) => todo!(),

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
        if !node_manager.skip_defaults {
            node_manager.initialize_defaults(ctx).await?;
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveness checks.
        node_manager
            .start_echoer_service_impl(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        ctx.start_worker(
            DefaultAddress::RPC_PROXY,
            RpcProxyService::new(node_manager.flow_controls.clone()),
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
