//! Node Manager (Node Man, the superhero that we deserve)

use miette::IntoDiagnostic;
use std::collections::BTreeMap;
use std::error::Error as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use minicbor::{Decoder, Encode};

pub use node_identities::*;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::CredentialsServerModule;
use ockam::identity::TrustContext;
use ockam::identity::Vault;
use ockam::identity::{
    Credentials, CredentialsServer, Identities, IdentitiesRepository, IdentityAttributesReader,
};
use ockam::identity::{Identifier, SecureChannels};
use ockam::{
    Address, Context, RelayService, RelayServiceOptions, Result, Routed, TcpTransport, Worker,
};
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, PolicyStorage, Resource};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_core::flow_control::FlowControlId;
use ockam_core::IncomingAccessControl;
use ockam_core::{AllowAll, AsyncTryClone};
use ockam_multiaddr::MultiAddr;

use crate::bootstrapped_identities_store::BootstrapedIdentityStore;
use crate::bootstrapped_identities_store::PreTrustedIdentities;
use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::cloud::{AuthorityNode, ProjectNode};
use crate::config::cli::TrustContextConfig;
use crate::config::lookup::ProjectLookup;
use crate::error::ApiError;
use crate::nodes::connection::{
    Connection, ConnectionBuilder, PlainTcpInstantiator, ProjectInstantiator,
    SecureChannelInstantiator,
};
use crate::nodes::models::base::NodeStatus;
use crate::nodes::models::portal::{OutletList, OutletStatus};
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::models::workers::{WorkerList, WorkerStatus};
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::{InMemoryNode, NODEMANAGER_ADDR};
use crate::DefaultAddress;

use super::registry::Registry;

pub(crate) mod background_node;
pub(crate) mod credentials;
mod flow_controls;
pub(crate) mod in_memory_node;
pub mod message;
mod node_identities;
mod node_services;
mod policy;
mod portals;
pub mod relay;
mod secure_channel;
mod transport;

const TARGET: &str = "ockam_api::nodemanager::service";

pub(crate) type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

pub(crate) fn encode_response<T: Encode<()>>(
    res: std::result::Result<Response<T>, Response<ockam_core::api::Error>>,
) -> Result<Vec<u8>> {
    let v = match res {
        Ok(r) => r.to_vec()?,
        Err(e) => e.to_vec()?,
    };

    Ok(v)
}

/// Node manager provides high-level operations to
///  - send messages
///  - create secure channels, inlet, outlet
///  - configure the trust context
///  - manage persistent data
pub struct NodeManager {
    pub(crate) cli_state: CliState,
    node_name: String,
    api_transport_flow_control_id: FlowControlId,
    pub(crate) tcp_transport: TcpTransport,
    enable_credential_checks: bool,
    identifier: Identifier,
    pub(crate) secure_channels: Arc<SecureChannels>,
    trust_context: Option<TrustContext>,
    pub(crate) registry: Registry,
    policies: Arc<dyn PolicyStorage>,
}

impl NodeManager {
    pub(super) fn identifier(&self) -> &Identifier {
        &self.identifier
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

    pub fn tcp_transport(&self) -> &TcpTransport {
        &self.tcp_transport
    }

    pub async fn list_outlets(&self) -> OutletList {
        OutletList::new(
            self.registry
                .outlets
                .entries()
                .await
                .iter()
                .map(|(alias, info)| {
                    OutletStatus::new(info.socket_addr, info.worker_addr.clone(), alias, None)
                })
                .collect(),
        )
    }

    /// Delete the cli state related to the current node when launched in-memory
    pub fn delete_node(&self) -> Result<()> {
        Ok(self
            .cli_state
            .nodes
            .delete_sigkill(self.node_name().as_str(), false)?)
    }
}

impl NodeManager {
    pub async fn create_authority_client(
        &self,
        authority_identifier: &Identifier,
        authority_multiaddr: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<AuthorityNode> {
        self.make_authority_node_client(
            authority_identifier,
            authority_multiaddr,
            &self
                .get_identifier(caller_identity_name)
                .await
                .into_diagnostic()?,
        )
        .await
        .into_diagnostic()
    }

    pub async fn create_project_client(
        &self,
        project_identifier: &Identifier,
        project_multiaddr: &MultiAddr,
        caller_identity_name: Option<String>,
    ) -> miette::Result<ProjectNode> {
        self.make_project_node_client(
            project_identifier,
            project_multiaddr,
            &self
                .get_identifier(caller_identity_name)
                .await
                .into_diagnostic()?,
        )
        .await
        .into_diagnostic()
    }
}

#[derive(Clone)]
pub struct NodeManagerWorker {
    pub node_manager: Arc<InMemoryNode>,
}

impl NodeManagerWorker {
    pub fn new(node_manager: Arc<InMemoryNode>) -> Self {
        NodeManagerWorker { node_manager }
    }

    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        self.node_manager.stop(ctx).await?;
        ctx.stop_worker(NODEMANAGER_ADDR).await?;
        Ok(())
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
    pre_trusted_identities: Option<PreTrustedIdentities>,
    start_default_services: bool,
    persistent: bool,
}

impl NodeManagerGeneralOptions {
    pub fn new(
        cli_state: CliState,
        node_name: String,
        pre_trusted_identities: Option<PreTrustedIdentities>,
        start_default_services: bool,
        persistent: bool,
    ) -> Self {
        Self {
            cli_state,
            node_name,
            pre_trusted_identities,
            start_default_services,
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

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            api_transport_flow_control_id: transport_options.api_transport_flow_control_id,
            tcp_transport: transport_options.tcp_transport,
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
            policies,
        };

        if let Some(tc) = trust_options.trust_context_config {
            debug!("configuring trust context");
            s.configure_trust_context(&tc).await?;
        }

        s.initialize_services(ctx, general_options.start_default_services)
            .await?;
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

    async fn initialize_default_services(
        &self,
        ctx: &Context,
        api_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        // Start services
        ctx.flow_controls()
            .add_consumer(DefaultAddress::UPPERCASE_SERVICE, api_flow_control_id);
        self.start_uppercase_service_impl(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
            .await?;

        RelayService::create(
            ctx,
            DefaultAddress::RELAY_SERVICE,
            RelayServiceOptions::new()
                .service_as_consumer(api_flow_control_id)
                .relay_as_consumer(api_flow_control_id),
        )
        .await?;

        self.create_secure_channel_listener(
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

    async fn initialize_services(
        &mut self,
        ctx: &Context,
        start_default_services: bool,
    ) -> Result<()> {
        let api_flow_control_id = self.api_transport_flow_control_id.clone();

        if start_default_services {
            self.initialize_default_services(ctx, &api_flow_control_id)
                .await?;
        }

        // Always start the echoer service as ockam_api::Medic assumes it will be
        // started unconditionally on every node. It's used for liveliness checks.
        ctx.flow_controls()
            .add_consumer(DefaultAddress::ECHO_SERVICE, &api_flow_control_id);
        self.start_echoer_service_impl(ctx, DefaultAddress::ECHO_SERVICE.into())
            .await?;

        Ok(())
    }

    pub async fn make_connection(
        &self,
        ctx: Arc<Context>,
        addr: &MultiAddr,
        identifier: Option<Identifier>,
        authorized: Option<Identifier>,
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> Result<Connection> {
        let identifier = match identifier {
            Some(identifier) => identifier,
            None => self.get_identifier(None).await?,
        };
        let authorized = authorized.map(|authorized| vec![authorized]);
        self.connect(ctx, addr, identifier, authorized, credential, timeout)
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
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> Result<Connection> {
        debug!(?timeout, "connecting to {}", &addr);
        let connection = ConnectionBuilder::new(addr.clone())
            .instantiate(
                ctx.clone(),
                self,
                ProjectInstantiator::new(identifier.clone(), timeout, credential.clone()),
            )
            .await?
            .instantiate(ctx.clone(), self, PlainTcpInstantiator::new())
            .await?
            .instantiate(
                ctx.clone(),
                self,
                SecureChannelInstantiator::new(&identifier, credential, timeout, authorized),
            )
            .await?
            .build();
        connection.add_default_consumers(ctx);

        debug!("connected to {connection:?}");
        Ok(connection)
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
                let node_name = &self.node_manager.node_name();
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
                encode_response(self.get_tcp_connection(req, address.to_string()).await)?
            }
            (Post, ["node", "tcp", "connection"]) => {
                encode_response(self.create_tcp_connection(req, dec, ctx).await)?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                encode_response(self.delete_tcp_connection(req, dec).await)?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => self.get_tcp_listeners(req).await.to_vec()?,
            (Get, ["node", "tcp", "listener", address]) => {
                encode_response(self.get_tcp_listener(req, address.to_string()).await)?
            }
            (Post, ["node", "tcp", "listener"]) => {
                encode_response(self.create_tcp_listener(req, dec).await)?
            }
            (Delete, ["node", "tcp", "listener"]) => {
                encode_response(self.delete_tcp_listener(req, dec).await)?
            }

            // ==*== Credential ==*==
            (Post, ["node", "credentials", "actions", "get"]) => self
                .get_credential(req, dec, ctx)
                .await?
                .either(Response::to_vec, Response::to_vec)?,
            (Post, ["node", "credentials", "actions", "present"]) => {
                encode_response(self.present_credential(req, dec, ctx).await)?
            }

            // ==*== Secure channels ==*==
            (Get, ["node", "secure_channel"]) => self.list_secure_channels(req).await.to_vec()?,
            (Get, ["node", "secure_channel_listener"]) => {
                self.list_secure_channel_listener(req).await.to_vec()?
            }
            (Post, ["node", "secure_channel"]) => {
                encode_response(self.create_secure_channel(req, dec, ctx).await)?
            }
            (Delete, ["node", "secure_channel"]) => {
                encode_response(self.delete_secure_channel(req, dec, ctx).await)?
            }
            (Get, ["node", "show_secure_channel"]) => {
                encode_response(self.show_secure_channel(req, dec).await)?
            }
            (Post, ["node", "secure_channel_listener"]) => {
                encode_response(self.create_secure_channel_listener(req, dec, ctx).await)?
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
                encode_response(self.start_authenticated_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => {
                encode_response(self.start_uppercase_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => {
                encode_response(self.start_echoer_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                encode_response(self.start_hop_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::CREDENTIALS_SERVICE]) => {
                encode_response(self.start_credentials_service(ctx, req, dec).await)?
            }
            (Post, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => {
                self.start_kafka_outlet_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Outlet)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => {
                self.start_kafka_consumer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => encode_response(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Consumer)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => {
                self.start_kafka_producer_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => encode_response(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Producer)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => {
                self.start_kafka_direct_service(ctx, req, dec).await?
            }
            (Delete, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => encode_response(
                self.delete_kafka_service(ctx, req, dec, KafkaServiceKind::Direct)
                    .await,
            )?,
            (Get, ["node", "services"]) => self.list_services(req).await?,
            (Get, ["node", "services", service_type]) => {
                self.list_services_of_type(req, service_type).await?
            }

            // ==*== Relay commands ==*==
            // TODO: change the path to 'relay' instead of 'forwarder'
            (Get, ["node", "forwarder", remote_address]) => {
                encode_response(self.show_relay(req, remote_address).await)?
            }
            (Get, ["node", "forwarder"]) => encode_response(self.get_relays(req).await)?,
            (Delete, ["node", "forwarder", remote_address]) => {
                encode_response(self.delete_relay(ctx, req, remote_address).await)?
            }
            (Post, ["node", "forwarder"]) => {
                encode_response(self.create_relay(ctx, req, dec.decode()?).await)?
            }

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => self.get_inlets(req).await.to_vec()?,
            (Get, ["node", "inlet", alias]) => encode_response(self.show_inlet(req, alias).await)?,
            (Get, ["node", "outlet"]) => self.get_outlets(req).await.to_vec()?,
            (Get, ["node", "outlet", alias]) => {
                encode_response(self.show_outlet(req, alias).await)?
            }
            (Post, ["node", "inlet"]) => encode_response(self.create_inlet(req, dec, ctx).await)?,
            (Post, ["node", "outlet"]) => {
                encode_response(self.create_outlet(ctx, req, dec.decode()?).await)?
            }
            (Delete, ["node", "outlet", alias]) => {
                encode_response(self.delete_outlet(req, alias).await)?
            }
            (Delete, ["node", "inlet", alias]) => {
                encode_response(self.delete_inlet(req, alias).await)?
            }
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                encode_response(self.add_consumer(ctx, req, dec))?
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
            (Post, ["policy", resource, action]) => encode_response(
                self.node_manager
                    .add_policy(resource, action, req, dec)
                    .await,
            )?,
            (Get, ["policy", resource]) => {
                encode_response(self.node_manager.list_policies(req, resource).await)?
            }
            (Get, ["policy", resource, action]) => self
                .node_manager
                .get_policy(req, resource, action)
                .await?
                .either(Response::to_vec, Response::to_vec)?,
            (Delete, ["policy", resource, action]) => {
                encode_response(self.node_manager.del_policy(req, resource, action).await)?
            }

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => self.send_message(ctx, req, dec).await?,

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

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.node_manager.medic_handle.stop_medic(ctx).await
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
