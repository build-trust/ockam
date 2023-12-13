//! Node Manager (Node Man, the superhero that we deserve)

use std::collections::BTreeMap;
use std::error::Error as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use miette::IntoDiagnostic;
use minicbor::{Decoder, Encode};

use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::TrustContext;
use ockam::identity::{Credentials, CredentialsServer, Identities};
use ockam::identity::{CredentialsServerModule, IdentityAttributesRepository};
use ockam::identity::{Identifier, SecureChannels};
use ockam::{
    Address, Context, RelayService, RelayServiceOptions, Result, Routed, TcpTransport, Worker,
};
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::{Action, Env, Expr, Policy, Resource};
use ockam_core::api::{Method, RequestHeader, Response};
use ockam_core::compat::{string::String, sync::Arc};
use ockam_core::flow_control::FlowControlId;
use ockam_core::AllowAll;
use ockam_core::IncomingAccessControl;
use ockam_multiaddr::MultiAddr;

use crate::bootstrapped_identities_store::PreTrustedIdentities;
use crate::cli_state::CliState;
use crate::cli_state::NamedTrustContext;
use crate::cloud::{AuthorityNode, ProjectNode};
use crate::error::ApiError;
use crate::nodes::connection::{
    Connection, ConnectionBuilder, PlainTcpInstantiator, ProjectInstantiator,
    SecureChannelInstantiator,
};
use crate::nodes::models::portal::{OutletList, OutletStatus};
use crate::nodes::models::transport::{TransportMode, TransportType};
use crate::nodes::registry::KafkaServiceKind;
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::{InMemoryNode, NODEMANAGER_ADDR};
use crate::session::MedicHandle;

use super::registry::Registry;

pub mod actions;
pub(crate) mod background_node;
pub(crate) mod credentials;
pub mod default_address;
mod flow_controls;
pub(crate) mod in_memory_node;
pub mod kafka_services;
pub mod message;
mod node_services;
mod policy;
pub mod portals;
mod projects;
pub mod relay;
pub mod resources;
mod secure_channel;
mod transport;
pub mod workers;

const TARGET: &str = "ockam_api::nodemanager::service";

pub(crate) type Alias = String;

/// Generate a new alias for some user created extension
#[inline]
fn random_alias() -> String {
    Address::random_local().without_type().to_owned()
}

/// Append the request header to the Response and encode in vector format
pub(crate) fn encode_response<T: Encode<()>>(
    req: &RequestHeader,
    res: std::result::Result<Response<T>, Response<ockam_core::api::Error>>,
) -> Result<Vec<u8>> {
    let v = match res {
        Ok(r) => r.with_headers(req).to_vec()?,
        Err(e) => e.with_headers(req).to_vec()?,
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
    node_identifier: Identifier,
    api_transport_flow_control_id: FlowControlId,
    pub(crate) tcp_transport: TcpTransport,
    pub(crate) secure_channels: Arc<SecureChannels>,
    trust_context: Option<TrustContext>,
    pub(crate) registry: Registry,
    pub(crate) medic_handle: MedicHandle,
}

impl NodeManager {
    pub fn identifier(&self) -> Identifier {
        self.node_identifier.clone()
    }

    pub(crate) async fn get_identifier_by_name(
        &self,
        identity_name: Option<String>,
    ) -> Result<Identifier> {
        if let Some(name) = identity_name {
            Ok(self.cli_state.get_identifier_by_name(name.as_ref()).await?)
        } else {
            Ok(self.identifier())
        }
    }

    pub fn trust_context_id(&self) -> Option<String> {
        self.trust_context.clone().map(|tc| tc.id().to_string())
    }

    pub fn node_name(&self) -> String {
        self.node_name.clone()
    }

    pub(super) fn identities(&self) -> Arc<Identities> {
        self.secure_channels.identities()
    }

    pub(super) fn identity_attributes_repository(&self) -> Arc<dyn IdentityAttributesRepository> {
        self.identities().identity_attributes_repository().clone()
    }

    pub(super) fn credentials(&self) -> Arc<Credentials> {
        self.identities().credentials()
    }

    pub(super) fn credentials_service(&self) -> Arc<dyn CredentialsServer> {
        Arc::new(CredentialsServerModule::new(self.credentials()))
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

    /// Delete the current node data
    pub async fn delete_node(&self) -> Result<()> {
        self.cli_state.remove_node(&self.node_name).await?;
        Ok(())
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
                .get_identifier_by_name(caller_identity_name)
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
                .get_identifier_by_name(caller_identity_name)
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
        resource: &Resource,
        action: &Action,
        trust_context_id: Option<&str>,
        custom_default: Option<&Expr>,
    ) -> Result<Arc<dyn IncomingAccessControl>> {
        if let Some(tcid) = trust_context_id {
            // Populate environment with known attributes:
            let mut env = Env::new();
            env.put("resource.id", str(resource.as_str()));
            env.put("action.id", str(action.as_str()));
            env.put("resource.trust_context_id", str(tcid));

            // Check if a policy exists for (resource, action) and if not, then
            // create or use a default entry:
            if self.cli_state.get_policy(resource, action).await?.is_none() {
                let fallback = match custom_default {
                    Some(e) => e.clone(),
                    None => eq([
                        ident("resource.trust_context_id"),
                        ident("subject.trust_context_id"),
                    ]),
                };
                self.cli_state
                    .set_policy(resource, action, &Policy::new(fallback))
                    .await?;
            }
            let policy_access_control = self
                .cli_state
                .make_policy_access_control(resource, action, env)
                .await?;
            Ok(Arc::new(policy_access_control))
        } else {
            debug!(
                "no policy access control set for resource '{}' and action: '{}'",
                &resource, &action
            );
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
    trust_context: Option<NamedTrustContext>,
}

impl NodeManagerTrustOptions {
    pub fn new(trust_context: Option<NamedTrustContext>) -> Self {
        Self { trust_context }
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

        let secure_channels = cli_state
            .secure_channels(
                &general_options.node_name,
                general_options.pre_trusted_identities,
            )
            .await?;

        debug!("start the medic");
        let medic_handle = MedicHandle::start_medic(ctx).await?;

        debug!("create the trust context");
        let tcp_transport = transport_options.tcp_transport;
        let trust_context = match trust_options.trust_context {
            None => None,
            Some(tc) => Some(
                tc.trust_context(&tcp_transport, secure_channels.clone())
                    .await?,
            ),
        };

        debug!("retrieve the node identifier");
        let node_identifier = cli_state
            .get_node(&general_options.node_name)
            .await?
            .identifier();

        let mut s = Self {
            cli_state,
            node_name: general_options.node_name,
            node_identifier,
            api_transport_flow_control_id: transport_options.api_transport_flow_control_id,
            tcp_transport,
            secure_channels,
            trust_context,
            registry: Default::default(),
            medic_handle,
        };

        debug!("retrieve the node identifier");
        s.initialize_services(ctx, general_options.start_default_services)
            .await?;
        info!("created a node manager for the node: {}", s.node_name);

        Ok(s)
    }

    async fn initialize_default_services(
        &self,
        ctx: &Context,
        api_flow_control_id: &FlowControlId,
    ) -> Result<()> {
        // Start services
        ctx.flow_controls()
            .add_consumer(DefaultAddress::UPPERCASE_SERVICE, api_flow_control_id);
        self.start_uppercase_service(ctx, DefaultAddress::UPPERCASE_SERVICE.into())
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
            ctx,
        )
        .await?;

        // If we've been configured with a trust context, we can start Credential Exchange service
        if let Ok(tc) = self.trust_context() {
            self.start_credentials_service(
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
        credential: Option<CredentialAndPurposeKey>,
        timeout: Option<Duration>,
    ) -> Result<Connection> {
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
        let project = self.cli_state.get_project_by_name(name).await?;
        Ok((project.access_route()?, project.identifier()?))
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
            (Get, ["node"]) => encode_response(req, self.get_node_status(ctx).await)?,

            // ==*== Tcp Connection ==*==
            (Get, ["node", "tcp", "connection"]) => self.get_tcp_connections(req).await.to_vec()?,
            (Get, ["node", "tcp", "connection", address]) => {
                encode_response(req, self.get_tcp_connection(address.to_string()).await)?
            }
            (Post, ["node", "tcp", "connection"]) => {
                encode_response(req, self.create_tcp_connection(ctx, dec.decode()?).await)?
            }
            (Delete, ["node", "tcp", "connection"]) => {
                encode_response(req, self.delete_tcp_connection(dec.decode()?).await)?
            }

            // ==*== Tcp Listeners ==*==
            (Get, ["node", "tcp", "listener"]) => self.get_tcp_listeners(req).await.to_vec()?,
            (Get, ["node", "tcp", "listener", address]) => {
                encode_response(req, self.get_tcp_listener(address.to_string()).await)?
            }
            (Post, ["node", "tcp", "listener"]) => {
                encode_response(req, self.create_tcp_listener(dec.decode()?).await)?
            }
            (Delete, ["node", "tcp", "listener"]) => {
                encode_response(req, self.delete_tcp_listener(dec.decode()?).await)?
            }

            // ==*== Credential ==*==
            (Post, ["node", "credentials", "actions", "get"]) => {
                encode_response(req, self.get_credential(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "credentials", "actions", "present"]) => {
                encode_response(req, self.present_credential(ctx, dec.decode()?).await)?
            }

            // ==*== Secure channels ==*==
            (Get, ["node", "secure_channel"]) => {
                encode_response(req, self.list_secure_channels().await)?
            }
            (Get, ["node", "secure_channel_listener"]) => {
                encode_response(req, self.list_secure_channel_listener().await)?
            }
            (Post, ["node", "secure_channel"]) => {
                encode_response(req, self.create_secure_channel(dec.decode()?, ctx).await)?
            }
            (Delete, ["node", "secure_channel"]) => {
                encode_response(req, self.delete_secure_channel(dec.decode()?, ctx).await)?
            }
            (Get, ["node", "show_secure_channel"]) => {
                encode_response(req, self.show_secure_channel(dec.decode()?).await)?
            }
            (Post, ["node", "secure_channel_listener"]) => encode_response(
                req,
                self.create_secure_channel_listener(dec.decode()?, ctx)
                    .await,
            )?,
            (Delete, ["node", "secure_channel_listener"]) => encode_response(
                req,
                self.delete_secure_channel_listener(dec.decode()?, ctx)
                    .await,
            )?,
            (Get, ["node", "show_secure_channel_listener"]) => {
                encode_response(req, self.show_secure_channel_listener(dec.decode()?).await)?
            }

            // ==*== Services ==*==
            //<<<<<<< HEAD
            (Post, ["node", "services", DefaultAddress::AUTHENTICATED_SERVICE]) => encode_response(
                req,
                self.start_authenticated_service(ctx, dec.decode()?).await,
            )?,
            (Post, ["node", "services", DefaultAddress::UPPERCASE_SERVICE]) => {
                encode_response(req, self.start_uppercase_service(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "services", DefaultAddress::ECHO_SERVICE]) => {
                encode_response(req, self.start_echoer_service(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "services", DefaultAddress::HOP_SERVICE]) => {
                encode_response(req, self.start_hop_service(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "services", DefaultAddress::CREDENTIALS_SERVICE]) => encode_response(
                req,
                self.start_credentials_service(ctx, dec.decode()?).await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                req,
                self.start_kafka_outlet_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_OUTLET]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Outlet)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => encode_response(
                req,
                self.start_kafka_consumer_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_CONSUMER]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Consumer)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => encode_response(
                req,
                self.start_kafka_producer_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_PRODUCER]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Producer)
                    .await,
            )?,
            (Post, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => encode_response(
                req,
                self.start_kafka_direct_service(ctx, dec.decode()?).await,
            )?,
            (Delete, ["node", "services", DefaultAddress::KAFKA_DIRECT]) => encode_response(
                req,
                self.delete_kafka_service(ctx, dec.decode()?, KafkaServiceKind::Direct)
                    .await,
            )?,
            (Get, ["node", "services"]) => encode_response(req, self.list_services().await)?,
            (Get, ["node", "services", service_type]) => {
                encode_response(req, self.list_services_of_type(service_type).await)?
            }

            // ==*== Relay commands ==*==
            // TODO: change the path to 'relay' instead of 'forwarder'
            (Get, ["node", "forwarder", remote_address]) => {
                encode_response(req, self.show_relay(req, remote_address).await)?
            }
            (Get, ["node", "forwarder"]) => encode_response(req, self.get_relays(req).await)?,
            (Delete, ["node", "forwarder", remote_address]) => {
                encode_response(req, self.delete_relay(ctx, req, remote_address).await)?
            }
            (Post, ["node", "forwarder"]) => {
                encode_response(req, self.create_relay(ctx, req, dec.decode()?).await)?
            }

            // ==*== Inlets & Outlets ==*==
            (Get, ["node", "inlet"]) => encode_response(req, self.get_inlets().await)?,
            (Get, ["node", "inlet", alias]) => encode_response(req, self.show_inlet(alias).await)?,
            (Get, ["node", "outlet"]) => self.get_outlets(req).await.to_vec()?,
            (Get, ["node", "outlet", alias]) => {
                encode_response(req, self.show_outlet(alias).await)?
            }
            (Post, ["node", "inlet"]) => {
                encode_response(req, self.create_inlet(ctx, dec.decode()?).await)?
            }
            (Post, ["node", "outlet"]) => {
                encode_response(req, self.create_outlet(ctx, dec.decode()?).await)?
            }
            (Delete, ["node", "outlet", alias]) => {
                encode_response(req, self.delete_outlet(alias).await)?
            }
            (Delete, ["node", "inlet", alias]) => {
                encode_response(req, self.delete_inlet(alias).await)?
            }
            (Delete, ["node", "portal"]) => todo!(),

            // ==*== Flow Controls ==*==
            (Post, ["node", "flow_controls", "add_consumer"]) => {
                encode_response(req, self.add_consumer(ctx, dec.decode()?).await)?
            }

            // ==*== Workers ==*==
            (Get, ["node", "workers"]) => encode_response(req, self.list_workers(ctx).await)?,

            // ==*== Policies ==*==
            (Post, ["policy", resource, action]) => {
                encode_response(req, self.add_policy(resource, action, dec.decode()?).await)?
            }
            (Get, ["policy", resource, action]) => {
                encode_response(req, self.get_policy(resource, action).await)?
            }
            (Get, ["policy", resource]) => {
                encode_response(req, self.list_policies(resource).await)?
            }
            (Delete, ["policy", resource, action]) => {
                encode_response(req, self.delete_policy(resource, action).await)?
            }

            // ==*== Messages ==*==
            (Post, ["v0", "message"]) => {
                encode_response(req, self.send_message(ctx, dec.decode()?).await)?
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
