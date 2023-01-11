//! Node Manager (Node Man, the superhero that we deserve)

use minicbor::Decoder;

use ockam::compat::asynchronous::RwLock;
use ockam::{Address, Context, ForwardingService, Result, Routed, TcpTransport, Worker};
use ockam_core::api::{Error, Method, Request, Response, ResponseBuilder, Status};
use ockam_core::compat::{
    boxed::Box,
    string::String,
    sync::{Arc, Mutex},
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{AllowAll, AsyncTryClone};
use ockam_identity::{Identity, IdentityIdentifier, PublicIdentity, SecureChannelRegistry};
use ockam_multiaddr::proto::{Project, Secure};
use ockam_multiaddr::{MultiAddr, Protocol};
use ockam_node::tokio;
use ockam_node::tokio::task::JoinHandle;
use ockam_vault::Vault;
use std::collections::BTreeMap;
use std::error::Error as _;
use std::path::PathBuf;
use std::time::Duration;

use super::models::secure_channel::CredentialExchangeMode;
use super::registry::Registry;
use crate::authenticator::direct::types::OneTimeCode;
use crate::cli_state::CliState;
use crate::config::cli::AuthoritiesConfig;
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::lmdb::LmdbStorage;
use crate::nodes::models::base::NodeStatus;
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::session::util::starts_with_host_tcp_secure;
use crate::session::{Medic, Sessions};
use crate::{multiaddr_to_route, try_address_to_multiaddr, DefaultAddress};

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

pub(crate) struct AuthorityInfo {
    identity: PublicIdentity,
    addr: MultiAddr,
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeManager {
    node_name: String,
    api_transport_id: Alias,
    transports: BTreeMap<Alias, (TransportType, TransportMode, String)>,
    tcp_transport: TcpTransport,
    pub(crate) controller_identity_id: IdentityIdentifier,
    skip_defaults: bool,
    enable_credential_checks: bool,
    vault: Vault,
    identity: Identity<Vault>,
    project_id: Option<String>,
    projects: Arc<BTreeMap<String, ProjectLookup>>,
    authorities: Option<Authorities>,
    pub(crate) authenticated_storage: LmdbStorage,
    pub(crate) registry: Registry,
    pub(crate) secure_channel_registry: SecureChannelRegistry,
    sessions: Arc<Mutex<Sessions>>,
    medic: JoinHandle<Result<(), ockam_core::Error>>,
    policies: LmdbStorage,
    token: Option<OneTimeCode>,
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
    pub(crate) fn identity(&self) -> Result<&Identity<Vault>> {
        Ok(&self.identity)
    }

    pub(crate) fn vault(&self) -> Result<&Vault> {
        Ok(&self.vault)
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
}

pub struct NodeManagerGeneralOptions {
    node_name: String,
    skip_defaults: bool,
}

impl NodeManagerGeneralOptions {
    pub fn new(node_name: String, skip_defaults: bool) -> Self {
        Self {
            node_name,
            skip_defaults,
        }
    }
}

pub struct NodeManagerProjectsOptions<'a> {
    ac: Option<&'a AuthoritiesConfig>,
    project_id: Option<String>,
    projects: BTreeMap<String, ProjectLookup>,
    token: Option<OneTimeCode>,
}

impl<'a> NodeManagerProjectsOptions<'a> {
    pub fn new(
        ac: Option<&'a AuthoritiesConfig>,
        project_id: Option<String>,
        projects: BTreeMap<String, ProjectLookup>,
        token: Option<OneTimeCode>,
    ) -> Self {
        Self {
            ac,
            project_id,
            projects,
            token,
        }
    }
}

pub struct NodeManagerTransportOptions {
    api_transport: (TransportType, TransportMode, String),
    tcp_transport: TcpTransport,
}

impl NodeManagerTransportOptions {
    pub fn new(
        api_transport: (TransportType, TransportMode, String),
        tcp_transport: TcpTransport,
    ) -> Self {
        Self {
            api_transport,
            tcp_transport,
        }
    }
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

        let state = CliState::new()?.nodes.get(&general_options.node_name)?;

        let authenticated_storage = LmdbStorage::new(&state.authenticated_storage_path()).await?;
        let policies_storage = LmdbStorage::new(&state.policies_storage_path()).await?;

        let vault = state.config.vault().await?;
        let identity = state.config.identity(ctx).await?;

        let medic = Medic::new();
        let sessions = medic.sessions();

        let mut s = Self {
            node_name: general_options.node_name,
            api_transport_id,
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
            authenticated_storage,
            registry: Default::default(),
            medic: {
                let ctx = ctx.async_try_clone().await?;
                tokio::spawn(medic.start(ctx))
            },
            sessions,
            policies: policies_storage,
            token: projects_options.token,
            secure_channel_registry: SecureChannelRegistry::new(),
        };

        if !general_options.skip_defaults {
            if let Some(ac) = projects_options.ac {
                s.configure_authorities(ac).await?;
            }
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveness checks.
        s.start_echoer_service_impl(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        Ok(s)
    }

    async fn configure_authorities(&mut self, ac: &AuthoritiesConfig) -> Result<()> {
        let vault = self.vault()?;

        let mut v = Vec::new();

        for a in ac.authorities() {
            v.push(AuthorityInfo {
                identity: PublicIdentity::import(a.1.identity(), vault).await?,
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
            None, // Not checking identifiers here in favor of credentials check
        )
        .await?;

        // If we've been configured with authorities, we can start Credentials Exchange service
        if self.authorities().is_ok() {
            self.start_credentials_service_impl(DefaultAddress::CREDENTIAL_SERVICE.into(), false)
                .await?;
        }

        Ok(())
    }

    /// Resolve project ID (if any) and create secure channel.
    ///
    /// Returns the secure channel worker address (if any) and the remainder
    /// of the address argument.
    async fn connect(
        &mut self,
        addr: &MultiAddr,
        auth: Option<IdentityIdentifier>,
        timeout: Option<Duration>,
        ctx: &Context,
    ) -> Result<(MultiAddr, MultiAddr)> {
        if let Some(p) = addr.first() {
            if p.code() == Project::CODE {
                let p = p
                    .cast::<Project>()
                    .ok_or_else(|| ApiError::message("invalid project protocol in multiaddr"))?;
                let (a, i) = self.resolve_project(&p)?;
                debug!(addr = %a, "creating secure channel");
                let r =
                    multiaddr_to_route(&a).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
                let i = Some(vec![i]);
                let m = CredentialExchangeMode::Oneway;
                let w = self
                    .create_secure_channel_impl(r, i, m, timeout, None, ctx)
                    .await?;
                let a = MultiAddr::default().try_with(addr.iter().skip(1))?;
                return Ok((try_address_to_multiaddr(&w)?, a));
            }
        }

        if let Some(pos) = starts_with_host_tcp_secure(addr) {
            debug!(%addr, "creating secure channel");
            let (a, b) = addr.split(pos);
            let r = multiaddr_to_route(&a).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
            let i = auth.clone().map(|i| vec![i]);
            let m = CredentialExchangeMode::Mutual;
            let w = self
                .create_secure_channel_impl(r, i, m, timeout, None, ctx)
                .await?;
            return Ok((try_address_to_multiaddr(&w)?, b));
        }

        if Some(Secure::CODE) == addr.last().map(|p| p.code()) {
            debug!(%addr, "creating secure channel");
            let r =
                multiaddr_to_route(addr).ok_or_else(|| ApiError::generic("invalid multiaddr"))?;
            let i = auth.clone().map(|i| vec![i]);
            let m = CredentialExchangeMode::Mutual;
            let w = self
                .create_secure_channel_impl(r, i, m, timeout, None, ctx)
                .await?;
            return Ok((try_address_to_multiaddr(&w)?, MultiAddr::default()));
        }

        Ok((MultiAddr::default(), addr.clone()))
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
            // TODO: Get all tcp connections
            (Get, ["node", "tcp", "connection"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_tcp_con_or_list(req, &node_manager.transports, TransportMode::Connect)
                    .to_vec()?
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
            (Post, ["node", "tcp", "listener"]) => self.add_transport(req, dec).await?.to_vec()?,
            (Delete, ["node", "tcp", "listener"]) => {
                self.delete_transport(req, dec).await?.to_vec()?
            }

            // ==*== Credentials ==*==
            (Post, ["node", "credentials", "actions", "get"]) => self
                .get_credential(req, dec)
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
                .create_secure_channel_listener(req, dec)
                .await?
                .to_vec()?,

            // ==*== Services ==*==
            (Post, ["node", "services", "vault"]) => {
                self.start_vault_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "identity"]) => {
                self.start_identity_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "authenticated"]) => self
                .start_authenticated_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", "uppercase"]) => self
                .start_uppercase_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", "echo"]) => {
                self.start_echoer_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "hop"]) => {
                self.start_hop_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "authenticator"]) => self
                .start_authenticator_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", "verifier"]) => {
                self.start_verifier_service(ctx, req, dec).await?.to_vec()?
            }
            (Post, ["node", "services", "credentials"]) => self
                .start_credentials_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Post, ["node", "services", "okta_identity_provider"]) => self
                .start_okta_identity_provider_service(ctx, req, dec)
                .await?
                .to_vec()?,
            (Get, ["node", "services"]) => {
                let node_manager = self.node_manager.read().await;
                self.list_services(req, &node_manager.registry).to_vec()?
            }

            // ==*== Forwarder commands ==*==
            (Post, ["node", "forwarder"]) => self.create_forwarder(ctx, req.id(), dec).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_inlets(req, &node_manager.registry).to_vec()?
            }
            (Get, ["node", "outlet"]) => {
                let node_manager = self.node_manager.read().await;
                self.get_outlets(req, &node_manager.registry).to_vec()?
            }
            (Post, ["node", "inlet"]) => self.create_inlet(req, dec, ctx).await?.to_vec()?,
            (Post, ["node", "outlet"]) => self.create_outlet(req, dec).await?.to_vec()?,
            (Delete, ["node", "portal"]) => todo!(),

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

            // ==*== Project' enrollers ==*==
            (Post, ["v0", "project-enrollers", project_id]) => {
                self.add_project_enroller(ctx, dec, project_id).await?
            }
            (Get, ["v0", "project-enrollers", project_id]) => {
                self.list_project_enrollers(ctx, dec, project_id).await?
            }
            (Delete, ["v0", "project-enrollers", project_id, identity_id]) => {
                self.delete_project_enroller(ctx, dec, project_id, identity_id)
                    .await?
            }

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
                    .body(format!("Invalid endpoint: {}", path))
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
        let mut node_manger = self.node_manager.write().await;
        if !node_manger.skip_defaults {
            node_manger.initialize_defaults(ctx).await?;
        }

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
                let err =
                    Error::new(req.path()).with_message(format!("failed to handle request: {err}"));
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
