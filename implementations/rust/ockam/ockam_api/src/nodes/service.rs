//! Node Manager (Node Man, the superhero that we deserve)

use std::collections::BTreeMap;
use std::error::Error as _;
use std::net::SocketAddr;
use std::path::PathBuf;

use minicbor::{Decoder, Encode};

pub use node_identities::*;
use ockam::identity::Vault;
use ockam::identity::{
    Credentials, CredentialsServer, Identities, IdentitiesRepository, IdentityAttributesReader,
};
use ockam::identity::{CredentialsServerModule, TrustContext};
use ockam::identity::{Identifier, SecureChannels};
use ockam::{
    Address, Context, ForwardingService, ForwardingServiceOptions, Result, Routed, TcpTransport,
    Worker,
};
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, PolicyStorage, Resource};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_core::flow_control::FlowControlId;
use ockam_core::IncomingAccessControl;
use ockam_core::{AllowAll, AsyncTryClone};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::RwLock;

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
use crate::nodes::models::portal::{OutletList, OutletStatus};
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::NODEMANAGER_ADDR;
use crate::session::sessions::{Key, Session};
use crate::session::MedicHandle;
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

pub(crate) fn encode_request_result<T: Encode<()>>(
    res: std::result::Result<Response<T>, Response<ockam_core::api::Error>>,
) -> Result<Vec<u8>> {
    let v = match res {
        Ok(r) => r.to_vec()?,
        Err(e) => e.to_vec()?,
    };

    Ok(v)
}

/// Node manager provides a messaging API to interact with the current node
pub struct NodeManager {
    pub(crate) cli_state: CliState,
    node_name: String,
    api_transport_flow_control_id: FlowControlId,
    pub(crate) tcp_transport: TcpTransport,
    pub(crate) controller_identity_id: Identifier,
    skip_defaults: bool,
    enable_credential_checks: bool,
    identifier: Identifier,
    pub(crate) secure_channels: Arc<SecureChannels>,
    trust_context: Option<TrustContext>,
    pub(crate) registry: Registry,
    medic_handle: MedicHandle,
    policies: Arc<dyn PolicyStorage>,
}

impl NodeManager {
    pub(super) fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    pub fn node_name(&self) -> String {
        self.node_name.clone()
    }

    pub(super) fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    pub(super) fn identities_repository(&self) -> Arc<dyn IdentitiesRepository> {
        self.identities().repository().clone()
    }

    pub(super) fn attributes_reader(&self) -> Arc<dyn IdentityAttributesReader> {
        self.identities_repository().as_attributes_reader()
    }

    pub(super) fn credentials(&self) -> Arc<Credentials> {
        self.identities().credentials()
    }

    pub(super) fn credentials_service(&self) -> Arc<dyn CredentialsServer> {
        Arc::new(CredentialsServerModule::new(self.credentials()))
    }

    pub(super) fn secure_channels_vault(&self) -> Vault {
        self.secure_channels.identities().vault()
    }

    pub(super) fn list_outlets(&self) -> OutletList {
        let outlets = self.registry.outlets.clone();
        OutletList::new(
            outlets
                .iter()
                .map(|(alias, info)| {
                    OutletStatus::new(info.socket_addr, info.worker_addr.clone(), alias, None)
                })
                .collect(),
        )
    }
}

#[derive(Clone)]
pub struct NodeManagerWorker {
    node_manager: Arc<RwLock<NodeManager>>,
}

impl NodeManagerWorker {
    pub fn new(node_manager: NodeManager) -> Self {
        NodeManagerWorker {
            node_manager: Arc::new(RwLock::new(node_manager)),
        }
    }

    pub fn inner(&self) -> &Arc<RwLock<NodeManager>> {
        &self.node_manager
    }

    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        let nm = self.node_manager.read().await;
        nm.medic_handle.stop_medic(ctx).await?;
        for addr in DefaultAddress::iter() {
            ctx.stop_worker(addr).await?;
        }
        ctx.stop_worker(NODEMANAGER_ADDR).await?;
        Ok(())
    }

    pub async fn controller_address(&self) -> MultiAddr {
        let node_manager = self.node_manager.read().await;
        node_manager.controller_address()
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
            env.put("resource.trust_context_id", str(tcid));

            // Check if a policy exists for (resource, action) and if not, then
            // create or use a default entry:
            if self.policies.get_policy(r, a).await?.is_none() {
                let fallback = match custom_default {
                    Some(e) => e.clone(),
                    None => eq([
                        ident("resource.trust_context_id"),
                        ident("subject.trust_context_id"),
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
            .ok_or_else(|| ApiError::core("Trust context doesn't exist"))
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
        transport_options: NodeManagerTransportOptions,
        trust_options: NodeManagerTrustOptions,
    ) -> Result<Self> {
        debug!("create transports");
        let api_transport_id = random_alias();
        let mut transports = BTreeMap::new();
        transports.insert(
            api_transport_id.clone(),
            transport_options.api_transport_flow_control_id.clone(),
        );

        debug!("create the identity repository");
        let cli_state = general_options.cli_state;
        let node_state = cli_state.nodes.get(&general_options.node_name)?;

        let repository: Arc<dyn IdentitiesRepository> =
            cli_state.identities.identities_repository().await?;

        //TODO: fix this.  Either don't require it to be a bootstrappedidentitystore (and use the
        //trait instead),  or pass it from the general_options always.
        let vault: Vault = node_state.config().vault().await?;
        let identities_repository: Arc<dyn IdentitiesRepository> =
            Arc::new(match general_options.pre_trusted_identities {
                None => BootstrapedIdentityStore::new(
                    Arc::new(PreTrustedIdentities::new_from_string("{}")?),
                    repository.clone(),
                ),
                Some(f) => BootstrapedIdentityStore::new(Arc::new(f), repository.clone()),
            });

        debug!("create the secure channels service");
        let secure_channels = SecureChannels::builder()
            .with_vault(vault)
            .with_identities_repository(identities_repository.clone())
            .build();

        let policies: Arc<dyn PolicyStorage> = Arc::new(node_state.policies_storage().await?);

        debug!("start the Medic");
        let medic_handle = MedicHandle::start_medic(ctx).await?;

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            api_transport_flow_control_id: transport_options.api_transport_flow_control_id,
            tcp_transport: transport_options.tcp_transport,
            controller_identity_id: Self::load_controller_identifier()?,
            skip_defaults: general_options.skip_defaults,
            enable_credential_checks: trust_options.trust_context_config.is_some()
                && trust_options
                    .trust_context_config
                    .as_ref()
                    .unwrap()
                    .authority()
                    .is_ok(),
            identifier: node_state.config().identifier()?,
            secure_channels,
            trust_context: None,
            registry: Default::default(),
            medic_handle,
            policies,
        };

        if !general_options.skip_defaults {
            debug!("starting default services");
            if let Some(tc) = trust_options.trust_context_config {
                debug!("configuring trust context");
                s.configure_trust_context(&tc).await?;
            }
        }
        info!("created a node manager for the node: {}", s.node_name);

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
        ctx.flow_controls()
            .add_consumer(DefaultAddress::UPPERCASE_SERVICE, api_flow_control_id);
        self.start_uppercase_service_impl(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
            .await?;

        ForwardingService::create(
            ctx,
            DefaultAddress::FORWARDING_SERVICE,
            ForwardingServiceOptions::new()
                .service_as_consumer(api_flow_control_id)
                .forwarder_as_consumer(api_flow_control_id),
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

    pub(crate) async fn resolve_project(&self, name: &str) -> Result<(MultiAddr, Identifier)> {
        let projects = ProjectLookup::from_state(self.cli_state.projects.list()?)
            .await
            .map_err(|e| ApiError::core(format!("Cannot load projects: {:?}", e)))?;
        if let Some(info) = projects.get(name) {
            let node_route = info
                .node_route
                .as_ref()
                .ok_or_else(|| ApiError::core("Project should have node route set"))?
                .clone();
            let identity_id = info
                .identity_id
                .as_ref()
                .ok_or_else(|| ApiError::core("Project should have identity set"))?
                .clone();
            Ok((node_route, identity_id))
        } else {
            Err(ApiError::core(format!("project {name} not found")))
        }
    }

    pub fn add_session(&self, session: Session) -> Key {
        self.medic_handle.add_session(session)
    }
}

impl NodeManagerWorker {
    //////// Request matching and response handling ////////

    async fn handle_request(
        &mut self,
        ctx: &mut Context,
        req: &RequestHeader,
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
                let node_name = &self.node_manager.read().await.node_name;
                Response::ok(req)
                    .body(NodeStatus::new(
                        node_name,
                        "Running",
                        ctx.list_workers().await?.len() as u32,
                        std::process::id() as i32,
                    ))
                    .to_vec()?
            }

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => self.get_tcp_connections(req).await.to_vec()?,
            (Get, ["node", "tcp", "connection", address]) => {
                encode_request_result(self.get_tcp_connection(req, address.to_string()).await)?
            }
            (Post, ["node", "tcp", "connection"]) => {
                encode_request_result(self.create_tcp_connection(req, dec, ctx).await)?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                encode_request_result(self.delete_tcp_connection(req, dec).await)?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => self.get_tcp_listeners(req).await.to_vec()?,
            (Get, ["node", "tcp", "listener", address]) => {
                encode_request_result(self.get_tcp_listener(req, address.to_string()).await)?
            }
            (Post, ["node", "tcp", "listener"]) => {
                encode_request_result(self.create_tcp_listener(req, dec).await)?
            }
            (Delete, ["node", "tcp", "listener"]) => {
                encode_request_result(self.delete_tcp_listener(req, dec).await)?
            }

            // ==*== Credential ==*==
            (Post, ["node", "credentials", "actions", "get"]) => self
                .get_credential(req, dec, ctx)
                .await?
                .either(Response::to_vec, Response::to_vec)?,
            (Post, ["node", "credentials", "actions", "present"]) => {
                encode_request_result(self.present_credential(req, dec, ctx).await)?
            }

            // ==*== Secure channels ==*==
            (Get, ["node", "secure_channel"]) => self.list_secure_channels(req).await.to_vec()?,
            (Get, ["node", "secure_channel_listener"]) => {
                self.list_secure_channel_listener(req).await.to_vec()?
            }
            (Post, ["node", "secure_channel"]) => {
                encode_request_result(self.create_secure_channel(req, dec, ctx).await)?
            }
            (Delete, ["node", "secure_channel"]) => {
                encode_request_result(self.delete_secure_channel(req, dec, ctx).await)?
            }
            (Get, ["node", "show_secure_channel"]) => {
                encode_request_result(self.show_secure_channel(req, dec).await)?
            }
            (Post, ["node", "secure_channel_listener"]) => {
                encode_request_result(self.create_secure_channel_listener(req, dec, ctx).await)?
            }
            (Delete, ["node", "secure_channel_listener"]) => self
                .delete_secure_channel_listener(ctx, req, dec)
                .await?
                .to_vec(),
            (Get, ["node", "show_secure_channel_listener"]) => {
                self.show_secure_channel_listener(req, dec).await?
            }

            // ==*== Services ==*==
            (Post, ["node", "services", DefaultAddress::AUTHENTICATED_SERVICE]) => {
                encode_request_result(self.start_authenticated_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => {
                encode_request_result(self.start_uppercase_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => {
                encode_request_result(self.start_echoer_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                encode_request_result(self.start_hop_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::CREDENTIALS_SERVICE]) => {
                encode_request_result(self.start_credentials_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => {
                self.start_kafka_outlet_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_request_result(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Outlet)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => {
                self.start_kafka_consumer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => {
                encode_request_result(
                    self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Consumer)
                        .await,
                )?
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => {
                self.start_kafka_producer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => {
                encode_request_result(
                    self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Producer)
                        .await,
                )?
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => {
                self.start_kafka_direct_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => encode_request_result(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Direct)
                    .await,
            )?,
            (Get, ["node", "services"]) => self.list_services(req).await?,
            (Get, ["node", "services", service_type]) => {
                self.list_services_of_type(req, service_type).await?
            }

            // ==*== Forwarder commands ==*==
            (Get, ["node", "forwarder", remote_address]) => {
                encode_request_result(self.show_forwarder(req, remote_address).await)?
            }
            (Get, ["node", "forwarder"]) => self.get_forwarders_response(req).await.to_vec()?,
            (Delete, ["node", "forwarder", remote_address]) => {
                encode_request_result(self.delete_forwarder(ctx, req, remote_address).await)?
            }
            (Post, ["node", "forwarder"]) => self.create_forwarder_response(ctx, req, dec).await?,

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => self.get_inlets(req).await.to_vec()?,
            (Get, ["node", "inlet", alias]) => {
                encode_request_result(self.show_inlet(req, alias).await)?
            }
            (Get, ["node", "outlet"]) => self.get_outlets(req).await.to_vec()?,
            (Get, ["node", "outlet", alias]) => {
                encode_request_result(self.show_outlet(req, alias).await)?
            }
            (Post, ["node", "inlet"]) => {
                encode_request_result(self.create_inlet(req, dec, ctx).await)?
            }
            (Post, ["node", "outlet"]) => {
                encode_request_result(self.create_outlet(ctx, req, dec.decode()?).await)?
            }
            (Delete, ["node", "outlet", alias]) => {
                encode_request_result(self.delete_outlet(req, alias).await)?
            }
            (Delete, ["node", "inlet", alias]) => {
                encode_request_result(self.delete_inlet(req, alias).await)?
            }
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                encode_request_result(self.add_consumer(ctx, req, dec))?
            }

            // ==*== Workers ==*==
            (Get, ["node", "workers"]) => {
                let workers = ctx.list_workers().await?;

                let mut list = Vec::new();
                workers
                    .iter()
                    .for_each(|addr| list.push(WorkerStatus::new(addr.address())));

                Response::ok(req).body(WorkerList::new(list)).to_vec()?
            }
            (Post, ["policy", resource, action]) => encode_request_result(
                self.node_manager
                    .read()
                    .await
                    .add_policy(resource, action, req, dec)
                    .await,
            )?,
            (Get, ["policy", resource]) => encode_request_result(
                self.node_manager
                    .read()
                    .await
                    .list_policies(req, resource)
                    .await,
            )?,
            (Get, ["policy", resource, action]) => self
                .node_manager
                .read()
                .await
                .get_policy(req, resource, action)
                .await?
                .either(Response::to_vec, Response::to_vec)?,
            (Delete, ["policy", resource, action]) => encode_request_result(
                self.node_manager
                    .read()
                    .await
                    .del_policy(req, resource, action)
                    .await,
            )?,

            // ==*== Spaces ==*==
            (Post, ["v0", "spaces"]) => self.create_space_response(ctx, dec.decode()?).await?,
            (Get, ["v0", "spaces"]) => self.list_spaces_response(ctx).await?,
            (Get, ["v0", "spaces", id]) => self.get_space_response(ctx, id).await?,
            (Delete, ["v0", "spaces", id]) => self.delete_space_response(ctx, id).await?,

            // ==*== Projects ==*==
            (Post, ["v1", "spaces", space_id, "projects"]) => {
                self.create_project_response(ctx, dec.decode()?, space_id)
                    .await?
            }
            (Get, ["v0", "projects", "version_info"]) => {
                self.get_project_version_response(ctx).await?
            }
            (Get, ["v0", "projects"]) => self.list_projects_response(ctx).await?,
            (Get, ["v0", "projects", project_id]) => {
                self.get_project_response(ctx, project_id).await?
            }
            (Delete, ["v0", "projects", space_id, project_id]) => {
                self.delete_project_response(ctx, space_id, project_id)
                    .await?
            }

            // ==*== Enroll ==*==
            (Post, ["v0", "enroll", "auth0"]) => {
                self.enroll_auth0_response(ctx, dec.decode()?).await?
            }
            (Get, ["v0", "enroll", "token"]) => self.generate_enrollment_token(ctx, dec).await?,
            (Put, ["v0", "enroll", "token"]) => {
                self.authenticate_enrollment_token(ctx, dec).await?
            }

            // ==*== Addons ==*==
            (Get, [project_id, "addons"]) => self.list_addons(ctx, project_id).await?,
            (Post, ["v1", "projects", project_id, "configure_addon", addon_id]) => {
                self.configure_addon(ctx, dec, project_id, addon_id).await?
            }
            (Post, ["v1", "projects", project_id, "disable_addon"]) => {
                self.disable_addon(ctx, dec, project_id).await?
            }

            // ==*== Operations ==*==
            (Get, ["v1", "operations", operation_id]) => {
                self.get_operation(ctx, operation_id).await?
            }

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => self.send_message(ctx, req, dec).await?,

            // ==*== Shares and Invitations ==*==
            (Get, ["v0", "invitations"]) => self.list_shares_response(ctx, dec.decode()?).await?,
            (Get, ["v0", "invitations", id]) => self.show_invitation_response(ctx, id).await?,
            (Post, ["v0", "invitations"]) => {
                self.create_invitation_response(ctx, dec.decode()?).await?
            }
            (Post, ["v0", "invitations/service"]) => {
                self.create_service_invitation_response(ctx, dec.decode()?)
                    .await?
            }
            (Post, ["v0", "accept_invitation"]) => {
                self.accept_invitation_response(ctx, dec.decode()?).await?
            }

            // ==*== Catch-all for Unimplemented APIs ==*==
            _ => {
                warn!(%method, %path, "Called invalid endpoint");
                Response::bad_request(req, &format!("Invalid endpoint: {} {}", method, path))
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
        let api_flow_control_id = node_manager.api_transport_flow_control_id.clone();

        if !node_manager.skip_defaults {
            node_manager
                .initialize_defaults(ctx, &api_flow_control_id)
                .await?;
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveliness checks.
        ctx.flow_controls()
            .add_consumer(DefaultAddress::ECHO_SERVICE, &api_flow_control_id);
        node_manager
            .start_echoer_service_impl(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        ctx.flow_controls()
            .add_consumer(DefaultAddress::RPC_PROXY, &api_flow_control_id);
        ctx.start_worker(DefaultAddress::RPC_PROXY, RpcProxyService::new())
            .await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let node_manager = self.node_manager.read().await;
        node_manager.medic_handle.stop_medic(ctx).await
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Vec<u8>>) -> Result<()> {
        let mut dec = Decoder::new(msg.as_body());
        let req: RequestHeader = match dec.decode() {
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
                Response::internal_error(&req, &format!("failed to handle request: {err} {req:?}"))
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
