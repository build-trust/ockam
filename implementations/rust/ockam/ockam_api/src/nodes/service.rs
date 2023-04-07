//! Node Manager (Node Man, the superhero that we deserve)

use minicbor::Decoder;

use ockam::identity::{Identity, IdentityIdentifier, PublicIdentity};
use ockam::{Address, Context, ForwardingService, Result, Routed, TcpTransport, Worker};
use ockam_abac::PolicyStorage;
use ockam_core::api::{Error, Method, Request, Response, ResponseBuilder, Status};
use ockam_core::compat::{
    boxed::Box,
    string::String,
    sync::{Arc, Mutex},
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::sessions::{SessionId, SessionPolicy, Sessions as MessageFlowSessions};
use ockam_core::{route, AllowAll, AsyncTryClone};
use ockam_identity::authenticated_storage::{
    AuthenticatedAttributeStorage, AuthenticatedStorage, IdentityAttributeStorage,
};
use ockam_identity::credential::Credential;
use ockam_identity::IdentityVault;
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
use crate::config::cli::AuthoritiesConfig;
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
mod policy;
mod portals;
mod secure_channel;
mod services;
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

pub(crate) struct Authorities(Vec<AuthorityInfo>);

impl Authorities {
    pub fn new(authorities: Vec<AuthorityInfo>) -> Self {
        Self(authorities)
    }

    pub fn public_identities(&self) -> Vec<PublicIdentity> {
        self.0.iter().map(|x| x.identity.clone()).collect()
    }
}

impl AsRef<[AuthorityInfo]> for Authorities {
    fn as_ref(&self) -> &[AuthorityInfo] {
        self.0.as_ref()
    }
}

#[derive(Clone)]
pub(crate) struct AuthorityInfo {
    identity: PublicIdentity,
    addr: MultiAddr,
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
    vault: Arc<dyn IdentityVault>,
    pub(crate) identity: Arc<Identity>,
    project_id: Option<String>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    authorities: Option<Authorities>,
    pub(crate) registry: Registry,
    sessions: Arc<Mutex<Sessions>>,
    medic: JoinHandle<Result<(), ockam_core::Error>>,
    policies: Arc<dyn PolicyStorage>,
    attributes_storage: Arc<dyn IdentityAttributeStorage>,
    pub(crate) message_flow_sessions: MessageFlowSessions,
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
    pub(crate) fn vault(&self) -> Result<Arc<dyn IdentityVault>> {
        Ok(self.vault.clone())
    }

    pub(crate) fn authorities(&self) -> Result<&Authorities> {
        self.authorities
            .as_ref()
            .ok_or_else(|| ApiError::generic("Authorities don't exist"))
    }

    /// Available only for member nodes
    pub(crate) fn project_id(&self) -> Result<&str> {
        self.project_id
            .as_deref()
            .ok_or_else(|| ApiError::generic("Project id is not set"))
    }
    pub fn message_flow_sessions(&self) -> &MessageFlowSessions {
        &self.message_flow_sessions
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

pub struct NodeManagerProjectsOptions<'a> {
    ac: Option<&'a AuthoritiesConfig>,
    project_id: Option<String>,
    projects: BTreeMap<String, ProjectLookup>,
    credential: Option<Credential>,
}

impl<'a> NodeManagerProjectsOptions<'a> {
    pub fn new(
        ac: Option<&'a AuthoritiesConfig>,
        project_id: Option<String>,
        projects: BTreeMap<String, ProjectLookup>,
        credential: Option<Credential>,
    ) -> Self {
        Self {
            ac,
            project_id,
            projects,
            credential,
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
    pub worker_address: Address,
    /// SessionId
    pub session_id: SessionId,
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

pub(crate) struct ConnectResult {
    pub(crate) secure_channel: MultiAddr,
    pub(crate) suffix: MultiAddr,
    pub(crate) session_id: Option<SessionId>,
}

impl NodeManager {
    /// Create a new NodeManager with the node name from the ockam CLI
    pub async fn create(
        ctx: &Context,
        general_options: NodeManagerGeneralOptions,
        projects_options: NodeManagerProjectsOptions<'_>,
        transport_options: NodeManagerTransportOptions,
    ) -> Result<Self> {
        let api_transport_id = random_alias();
        let mut transports = BTreeMap::new();
        transports.insert(api_transport_id.clone(), transport_options.api_transport);

        let cli_state = general_options.cli_state;
        let node_state = cli_state.nodes.get(&general_options.node_name)?;

        let authenticated_storage: Arc<dyn AuthenticatedStorage> =
            cli_state.identities.authenticated_storage().await?;

        //TODO: fix this.  Either don't require it to be a bootstrappedidentitystore (and use the
        //trait instead),  or pass it from the general_options always.
        let attributes_storage: Arc<dyn IdentityAttributeStorage> =
            Arc::new(match general_options.pre_trusted_identities {
                None => BootstrapedIdentityStore::new(
                    Arc::new(PreTrustedIdentities::new_from_string("{}")?),
                    Arc::new(AuthenticatedAttributeStorage::new(
                        authenticated_storage.clone(),
                    )),
                ),
                Some(f) => BootstrapedIdentityStore::new(
                    Arc::new(f),
                    Arc::new(AuthenticatedAttributeStorage::new(
                        authenticated_storage.clone(),
                    )),
                ),
            });

        let policies: Arc<dyn PolicyStorage> = Arc::new(node_state.policies_storage().await?);

        let vault: Arc<dyn IdentityVault> = Arc::new(node_state.config.vault().await?);
        let identity = Arc::new(node_state.config.identity(ctx).await?);
        if let Some(cred) = projects_options.credential {
            identity.set_credential(cred.to_owned()).await;
        }

        let message_flow_sessions = ockam_core::sessions::Sessions::default();
        let medic = Medic::new(message_flow_sessions.clone());
        let sessions = medic.sessions();

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            transports,
            tcp_transport: transport_options.tcp_transport,
            controller_identity_id: Self::load_controller_identity_id()?,
            skip_defaults: general_options.skip_defaults,
            enable_credential_checks: projects_options.ac.is_some()
                && projects_options.project_id.is_some(),
            vault,
            identity,
            projects: Arc::new(projects_options.projects),
            project_id: projects_options.project_id,
            authorities: None,
            registry: Default::default(),
            medic: {
                let ctx = ctx.async_try_clone().await?;
                tokio::spawn(medic.start(ctx))
            },
            sessions,
            policies,
            attributes_storage,
            message_flow_sessions,
        };

        if !general_options.skip_defaults {
            if let Some(ac) = projects_options.ac {
                s.configure_authorities(ac).await?;
            }
        }

        Ok(s)
    }

    async fn configure_authorities(&mut self, ac: &AuthoritiesConfig) -> Result<()> {
        let vault = self.vault()?;

        let mut v = Vec::new();

        for a in ac.authorities() {
            v.push(AuthorityInfo {
                identity: PublicIdentity::import(a.1.identity(), vault.clone()).await?,
                addr: a.1.access_route().clone(),
            })
        }

        self.authorities = Some(Authorities::new(v));

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

        // If we've been configured with authorities, we can start Credential Exchange service
        if self.authorities().is_ok() {
            self.start_credentials_service_impl(DefaultAddress::CREDENTIALS_SERVICE.into(), false)
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
            if let Some(session_id) = &res.session_id {
                self.message_flow_sessions.add_consumer(
                    &DefaultAddress::SECURE_CHANNEL_LISTENER.into(),
                    session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
                self.message_flow_sessions.add_consumer(
                    &DefaultAddress::UPPERCASE_SERVICE.into(),
                    session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
                self.message_flow_sessions.add_consumer(
                    &DefaultAddress::ECHO_SERVICE.into(),
                    session_id,
                    SessionPolicy::ProducerAllowMultiple,
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
                let tcp_session = multiaddr_to_route(&a, transport, &self.message_flow_sessions)
                    .await
                    .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                let i = Some(vec![i]);
                let m = CredentialExchangeMode::Oneway;
                let (sc_address, sc_session_id) = self
                    .create_secure_channel_impl(
                        tcp_session.route,
                        i,
                        m,
                        timeout,
                        identity_name,
                        ctx,
                        credential_name,
                    )
                    .await?;
                let a = MultiAddr::default().try_with(addr.iter().skip(1))?;

                let res = ConnectResult {
                    secure_channel: try_address_to_multiaddr(&sc_address)?,
                    suffix: a,
                    session_id: Some(sc_session_id),
                };

                return Ok(res);
            }
        }

        if let Some(pos1) = starts_with_host_tcp(addr) {
            debug!(%addr, "creating a tcp connection");
            let (a1, b1) = addr.split(pos1);
            return match starts_with_secure(&b1) {
                Some(pos2) => {
                    let tcp_session =
                        multiaddr_to_route(&a1, transport, &self.message_flow_sessions)
                            .await
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                    debug!(%addr, "creating a secure channel");
                    let (a2, b2) = b1.split(pos2);
                    let m = CredentialExchangeMode::Mutual;
                    let r2 = local_multiaddr_to_route(&a2)
                        .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                    let (sc_address, sc_session_id) = self
                        .create_secure_channel_impl(
                            route![tcp_session.route, r2],
                            authorized_identities.clone(),
                            m,
                            timeout,
                            identity_name,
                            ctx,
                            credential_name,
                        )
                        .await?;

                    let res = ConnectResult {
                        secure_channel: try_address_to_multiaddr(&sc_address)?,
                        suffix: b2,
                        session_id: Some(sc_session_id),
                    };

                    Ok(res)
                }
                None => {
                    let tcp_session =
                        multiaddr_to_route(addr, transport, &self.message_flow_sessions)
                            .await
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;

                    let res = ConnectResult {
                        secure_channel: route_to_multiaddr(&tcp_session.route)
                            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?,
                        suffix: Default::default(),
                        session_id: tcp_session.session_id,
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
            let (sc_address, sc_session_id) = self
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
                session_id: Some(sc_session_id),
            };

            return Ok(res);
        }

        let res = ConnectResult {
            secure_channel: Default::default(),
            suffix: addr.clone(),
            session_id: None,
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
                self.present_credential(req, dec).await?.to_vec()?
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
                self.delete_secure_channel(req, dec).await?.to_vec()?
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
            "rpc_proxy_service",
            RpcProxyService::new(node_manager.message_flow_sessions.clone()),
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
