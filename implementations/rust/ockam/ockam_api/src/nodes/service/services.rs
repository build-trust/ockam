use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::hop::Hop;
use crate::identity::IdentityService;
use crate::kafka::{
    KafkaInletMap, KafkaPortalListener, KafkaSecureChannelController,
    KafkaSecureChannelControllerImpl, KAFKA_SECURE_CHANNEL_LISTENER_ADDRESS,
    ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS, ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS,
};
use crate::lmdb::LmdbStorage;
use crate::nodes::models::services::{
    ServiceList, ServiceStatus, StartAuthenticatedServiceRequest, StartAuthenticatorRequest,
    StartCredentialsService, StartEchoerServiceRequest, StartHopServiceRequest,
    StartIdentityServiceRequest, StartKafkaConsumerRequest, StartKafkaProducerRequest,
    StartOktaIdentityProviderRequest, StartServiceRequest, StartUppercaseServiceRequest,
    StartVaultServiceRequest, StartVerifierService,
};
use crate::nodes::registry::{
    CredentialsServiceInfo, KafkaServiceInfo, KafkaServiceKind, Registry, VerifierServiceInfo,
};
use crate::nodes::NodeManager;
use crate::port_range::PortRange;
use crate::session::{Replacer, Session};
use crate::uppercase::Uppercase;
use crate::vault::VaultService;
use crate::{
    actions, resources, route_to_multiaddr, try_address_to_multiaddr, try_multiaddr_to_route,
};
use crate::{multiaddr_to_route, DefaultAddress};
use core::time::Duration;
use minicbor::Decoder;
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::PolicyStorage;
use ockam_core::api::{bad_request, Error, Request, Response, ResponseBuilder};
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll};
use ockam_core::{Address, AsyncTryClone, Result};
use ockam_identity::access_control::IdentityIdAccessControl;
use ockam_identity::authenticated_storage::AuthenticatedStorage;
use ockam_identity::{
    Identity, IdentityIdentifier, IdentityVault, TrustEveryonePolicy, TrustIdentifierPolicy,
};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_vault::Vault;
use std::ops::Add;

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn start_vault_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.vault_services.contains_key(&addr) {
            return Err(ApiError::generic("Vault service exists at this address"));
        }

        let vault = self.vault()?.async_try_clone().await?;
        let service = VaultService::new(vault);

        ctx.start_worker(
            addr.clone(),
            service,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry
            .vault_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_identity_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.identity_services.contains_key(&addr) {
            return Err(ApiError::generic("Identity service exists at this address"));
        }

        let vault = self.vault()?.async_try_clone().await?;
        let service = IdentityService::new(ctx, vault).await?;

        ctx.start_worker(
            addr.clone(),
            service,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry
            .identity_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_credentials_service_impl<'a>(
        &mut self,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Credentials service exists at this address",
            ));
        }

        let identity = self.identity()?;

        let authorities = self.authorities()?;

        identity
            .start_credential_exchange_worker(
                authorities.public_identities(),
                addr.clone(),
                !oneway,
                self.attributes_storage.async_try_clone().await?,
            )
            .await?;

        self.registry
            .credentials_services
            .insert(addr, CredentialsServiceInfo::default());

        Ok(())
    }

    pub(super) async fn start_authenticated_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.authenticated_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Authenticated service exists at this address",
            ));
        }

        let s = self.attributes_storage.async_try_clone().await?;
        let server = Server::new(s);
        ctx.start_worker(
            addr.clone(),
            server,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry
            .authenticated_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_uppercase_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Uppercase service exists at this address",
            ));
        }

        ctx.start_worker(
            addr.clone(),
            Uppercase,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry
            .uppercase_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_echoer_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr) {
            return Err(ApiError::generic("Echoer service exists at this address"));
        }

        ctx.start_worker(
            addr.clone(),
            Echoer,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry
            .echoer_services
            .insert(addr, Default::default());

        Ok(())
    }

    pub(super) async fn start_hop_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.hop_services.contains_key(&addr) {
            return Err(ApiError::generic("Hop service exists at this address"));
        }

        ctx.start_worker(
            addr.clone(),
            Hop,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        self.registry.hop_services.insert(addr, Default::default());

        Ok(())
    }

    #[cfg(feature = "direct-authenticator")]
    pub(super) async fn start_direct_authenticator_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        enrollers: &str,
        reload_enrollers: bool,
        proj: &[u8],
    ) -> Result<()> {
        use crate::nodes::registry::AuthenticatorServiceInfo;
        if self.registry.authenticator_service.contains_key(&addr) {
            return Err(ApiError::generic("Authenticator service already started"));
        }
        let db = self.attributes_storage.async_try_clone().await?;
        let id = self.identity()?.async_try_clone().await?;
        let au = crate::authenticator::direct::Server::new(
            proj.to_vec(),
            db,
            enrollers,
            reload_enrollers,
            id,
        )
        .await?;
        ctx.start_worker(
            addr.clone(),
            au,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;
        self.registry
            .authenticator_service
            .insert(addr, AuthenticatorServiceInfo::default());
        Ok(())
    }

    pub(super) async fn start_okta_identity_provider_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        tenant_base_url: &str,
        certificate: &str,
        attributes: &[&str],
        proj: &[u8],
    ) -> Result<()> {
        use crate::nodes::registry::OktaIdentityProviderServiceInfo;
        if self
            .registry
            .okta_identity_provider_services
            .contains_key(&addr)
        {
            return Err(ApiError::generic(
                "Okta Identity Provider service already started",
            ));
        }
        let db = self.authenticated_storage.async_try_clone().await?;
        let au =
            crate::okta::Server::new(proj.to_vec(), db, tenant_base_url, certificate, attributes)?;
        ctx.start_worker(
            addr.clone(),
            au,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;
        self.registry
            .okta_identity_provider_services
            .insert(addr, OktaIdentityProviderServiceInfo::default());
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn start_kafka_service_impl<'a>(
        &mut self,
        context: &Context,
        node_manager: Arc<RwLock<NodeManager>>,
        project_name: String,
        local_interceptor_address: Address,
        bind_ip: String,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        project_route_multiaddr: MultiAddr,
        kind: KafkaServiceKind,
    ) -> Result<Replacer> {
        let local_identity = self.identity()?.async_try_clone().await?;
        let project_info = self.resolve_project(&project_name).unwrap();

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            local_identity.async_try_clone().await?,
            route![],
        );

        // override default policy to allow incoming packets from the project
        let (_addr, identity_identifier) = self.resolve_project(&project_name)?;
        self.policies
            .set_policy(
                &resources::INLET,
                &actions::HANDLE_MESSAGE,
                &eq([
                    ident("subject.identifier"),
                    str(identity_identifier.to_string()),
                ]),
            )
            .await?;

        if let KafkaServiceKind::Consumer = kind {
            secure_channel_controller
                .create_consumer_listener(context)
                .await?;

            self.create_secure_channel_listener_impl(
                Address::from_string(KAFKA_SECURE_CHANNEL_LISTENER_ADDRESS),
                None,
                None,
                None,
                context,
            )
            .await?;
        }

        let secure_channel_controller_trait = secure_channel_controller.into_trait();

        let only_from_project =
            Arc::new(IdentityIdAccessControl::new(vec![project_info.1.clone()]));

        let inlet_map = KafkaInletMap::new(
            self.tcp_transport.async_try_clone().await?,
            only_from_project,
            route![],
            bind_ip,
            server_bootstrap_port,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::message("invalid port range"))?,
        );

        KafkaPortalListener::create(
            context,
            secure_channel_controller_trait.clone(),
            local_interceptor_address.clone(),
            inlet_map.clone(),
        )
        .await?;

        let replacer = Self::kafka_connection_replacer(
            Arc::new(context.async_try_clone().await?),
            node_manager,
            inlet_map.clone(),
            secure_channel_controller_trait.clone(),
            Arc::new(local_identity),
            project_route_multiaddr,
            local_interceptor_address.clone(),
        );

        self.registry
            .kafka_services
            .insert(local_interceptor_address, KafkaServiceInfo::new(kind));

        //since we are holding the node manager lock which is needed by the replacer
        //we cannot call `replacer` it from within the method
        Ok(replacer)
    }

    fn kafka_connection_replacer<V: IdentityVault, S: AuthenticatedStorage>(
        context: Arc<Context>,
        node_manager: Arc<RwLock<NodeManager>>,
        inlet_map: KafkaInletMap,
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        local_identity: Arc<Identity<V, S>>,
        project_route_multiaddr: MultiAddr,
        local_interceptor_address: Address,
    ) -> Replacer {
        Box::new(move |_old_multiaddr| {
            let inlet_map = inlet_map.clone();
            let context = context.clone();
            let local_identity = local_identity.clone();
            let local_interceptor_address = local_interceptor_address.clone();
            let secure_channel_controller = secure_channel_controller.clone();
            let node_manager = node_manager.clone();
            let project_route_multiaddr = project_route_multiaddr.clone();
            Box::pin(async move {
                debug!(
                    "replacer called for kafka service: {}",
                    project_route_multiaddr.to_string()
                );

                let mut node_manager = node_manager.write().await;
                let (maybe_tunnel_multiaddr, suffix_address) = node_manager
                    .connect(
                        &project_route_multiaddr,
                        Some(local_identity.identifier().clone()),
                        Some(Duration::from_secs(20)),
                        &context,
                    )
                    .await?;
                debug!("connected");

                let project_multiaddr = maybe_tunnel_multiaddr.try_with(&suffix_address)?;
                let project_route = try_multiaddr_to_route(&project_multiaddr)?;

                let interceptor_route = route![
                    local_interceptor_address,
                    project_route.clone(),
                    ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS
                ];

                debug!("project_route: {project_route:?}");
                debug!("interceptor_route: {interceptor_route:?}");

                //let's restart forwarders and close existing secure channels
                secure_channel_controller
                    .change_route(&context, interceptor_route.clone())
                    .await?;

                //and then rebuild every inlet
                inlet_map
                    .change_route(&context, interceptor_route.clone())
                    .await?;

                route_to_multiaddr(&project_route)
                    .ok_or_else(|| ApiError::generic("cannot convert route to multiaddr"))
            })
        })
    }
}

impl NodeManagerWorker {
    pub(super) async fn start_vault_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartVaultServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_vault_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_identity_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartIdentityServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_identity_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_authenticated_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartAuthenticatedServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager
            .start_authenticated_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_uppercase_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartUppercaseServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_uppercase_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_echoer_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartEchoerServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_echoer_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_hop_service(
        &mut self,
        ctx: &Context,
        req: &Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let req_body: StartHopServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        node_manager.start_hop_service_impl(ctx, addr).await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_authenticator_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        #[cfg(not(feature = "direct-authenticator"))]
        return Err(ApiError::generic("Direct authenticator not available"));

        #[cfg(feature = "direct-authenticator")]
        {
            let body: StartAuthenticatorRequest = dec.decode()?;
            let addr: Address = body.address().into();

            node_manager
                .start_direct_authenticator_service_impl(
                    ctx,
                    addr,
                    body.enrollers(),
                    body.reload_enrollers(),
                    body.project(),
                )
                .await?;
        }

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_okta_identity_provider_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartOktaIdentityProviderRequest = dec.decode()?;
        let addr: Address = body.address().into();
        node_manager
            .start_okta_identity_provider_service_impl(
                ctx,
                addr,
                body.tenant_base_url(),
                body.certificate(),
                body.attributes(),
                body.project(),
            )
            .await?;
        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_verifier_service<'a>(
        &mut self,
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartVerifierService = dec.decode()?;
        let addr: Address = body.address().into();

        if node_manager.registry.verifier_services.contains_key(&addr) {
            return Err(ApiError::generic("Verifier service exists at this address"));
        }

        let vault = node_manager.vault.async_try_clone().await?;
        let vs = crate::verifier::Verifier::new(vault);
        ctx.start_worker(
            addr.clone(),
            vs,
            AllowAll, // FIXME: @ac
            AllowAll,
        )
        .await?;

        node_manager
            .registry
            .verifier_services
            .insert(addr, VerifierServiceInfo::default());

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_credentials_service<'a>(
        &mut self,
        _ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();

        node_manager
            .start_credentials_service_impl(addr, oneway)
            .await?;

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_kafka_consumer_service<'a>(
        &mut self,
        context: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartServiceRequest<StartKafkaConsumerRequest> = dec.decode()?;
        let listener_address: Address = body.address().into();
        let body_req = body.request();

        let project_route = body_req.project_route().to_string().parse()?;
        let project_name = match self.extract_project(req, &project_route) {
            Ok(project_name) => project_name,
            Err(err) => {
                return Ok(err.to_vec()?);
            }
        };

        let mut replacer = node_manager
            .start_kafka_service_impl(
                context,
                self.node_manager.clone(),
                project_name,
                listener_address,
                body_req.bootstrap_server_ip().to_string(),
                body_req.bootstrap_server_port(),
                body_req.brokers_port_range(),
                project_route,
                KafkaServiceKind::Consumer,
            )
            .await?;

        drop(node_manager);

        //initialize the first connection
        let initial_multiaddr = replacer(MultiAddr::default()).await?;

        let mut session = Session::new(initial_multiaddr);
        //add a session with the replacer
        session.set_replacer(replacer);

        let mut node_manager = self.node_manager.write().await;
        node_manager.sessions.lock().unwrap().add(session);

        Ok(Response::ok(req.id()).to_vec()?)
    }

    pub(super) async fn start_kafka_producer_service<'a>(
        &mut self,
        context: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartServiceRequest<StartKafkaProducerRequest> = dec.decode()?;
        let listener_address: Address = body.address().into();
        let body_req = body.request();

        let project_route = body_req.project_route().to_string().parse()?;
        let project_name = match self.extract_project(req, &project_route) {
            Ok(project_name) => project_name,
            Err(err) => {
                return Ok(err.to_vec()?);
            }
        };

        let mut replacer = node_manager
            .start_kafka_service_impl(
                context,
                self.node_manager.clone(),
                project_name,
                listener_address,
                body_req.bootstrap_server_ip().to_string(),
                body_req.bootstrap_server_port(),
                body_req.brokers_port_range(),
                body_req.project_route().to_string().parse()?,
                KafkaServiceKind::Producer,
            )
            .await?;

        drop(node_manager);

        //initialize the first connection
        let initial_multiaddr = replacer(MultiAddr::default()).await?;

        let mut session = Session::new(initial_multiaddr);
        //add a session with the replacer
        session.set_replacer(replacer);

        let mut node_manager = self.node_manager.write().await;
        node_manager.sessions.lock().unwrap().add(session);

        Ok(Response::ok(req.id()).to_vec()?)
    }

    fn extract_project<'a>(
        &self,
        req: &'a Request<'_>,
        project_route_multiaddr: &MultiAddr,
    ) -> std::result::Result<String, ResponseBuilder<Error<'a>>> {
        project_route_multiaddr
            .first()
            .and_then(|value| value.cast::<Project>().map(|p| p.to_string()))
            .ok_or_else(|| bad_request(req, "invalid project route"))
    }

    pub(super) fn list_services<'a>(
        &self,
        req: &Request<'a>,
        registry: &'a Registry,
    ) -> ResponseBuilder<ServiceList<'a>> {
        let mut list = Vec::new();
        registry.vault_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::VAULT_SERVICE,
            ))
        });
        registry.identity_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::IDENTITY_SERVICE,
            ))
        });
        registry.authenticated_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::AUTHENTICATED_SERVICE,
            ))
        });
        registry.uppercase_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::UPPERCASE_SERVICE,
            ))
        });
        registry.echoer_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::ECHO_SERVICE,
            ))
        });
        registry.hop_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::HOP_SERVICE,
            ))
        });
        registry.verifier_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(addr.address(), DefaultAddress::VERIFIER))
        });
        registry.credentials_services.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::CREDENTIALS_SERVICE,
            ))
        });
        registry.kafka_services.iter().for_each(|(address, info)| {
            list.push(ServiceStatus::new(
                address.address(),
                match info.kind() {
                    KafkaServiceKind::Consumer => "kafka-consumer",
                    KafkaServiceKind::Producer => "kafka-producer",
                },
            ))
        });

        #[cfg(feature = "direct-authenticator")]
        registry.authenticator_service.keys().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::AUTHENTICATOR,
            ))
        });

        Response::ok(req.id()).body(ServiceList::new(list))
    }
}
