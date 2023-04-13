use std::net::IpAddr;

use minicbor::Decoder;

use ockam::{Address, Context, Result};
use ockam_abac::expr::{and, eq, ident, str};
use ockam_abac::{Action, Env, Expr, PolicyAccessControl, Resource};
use ockam_core::api::{bad_request, Error, Request, Response, ResponseBuilder};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::Arc;
use ockam_core::{route, AllowAll, IncomingAccessControl};
use ockam_identity::{identities, AuthorityService, CredentialsIssuer, TrustContext};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_node::WorkerBuilder;

use crate::auth::Server;
use crate::authenticator::direct::EnrollmentTokenAuthenticator;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::hop::Hop;
use crate::identity::IdentityService;
use crate::kafka::{
    KafkaInletController, KafkaPortalListener, KafkaSecureChannelControllerImpl,
    ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS, ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS,
};
use crate::nodes::models::portal::CreateInlet;
use crate::nodes::models::services::{
    ServiceList, ServiceStatus, StartAuthenticatedServiceRequest, StartAuthenticatorRequest,
    StartCredentialsService, StartEchoerServiceRequest, StartHopServiceRequest,
    StartIdentityServiceRequest, StartKafkaConsumerRequest, StartKafkaProducerRequest,
    StartOktaIdentityProviderRequest, StartServiceRequest, StartUppercaseServiceRequest,
    StartVerifierService,
};
use crate::nodes::registry::{
    AuthenticatorServiceInfo, CredentialsServiceInfo, KafkaServiceInfo, KafkaServiceKind, Registry,
    VerifierServiceInfo,
};
use crate::nodes::NodeManager;
use crate::port_range::PortRange;
use crate::uppercase::Uppercase;
use crate::DefaultAddress;
use crate::{actions, resources};

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn start_identity_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.identity_services.contains_key(&addr) {
            return Err(ApiError::generic("Identity service exists at this address"));
        }

        let service = IdentityService::new(self.node_identities()).await?;

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
        ctx: &Context,
        trust_context: TrustContext,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr) {
            return Err(ApiError::generic(
                "Credentials service exists at this address",
            ));
        }

        self.credentials_service()
            .start(ctx, trust_context, self.identifier(), addr.clone(), !oneway)
            .await?;

        self.registry
            .credentials_services
            .insert(addr.clone(), CredentialsServiceInfo::default());

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

        let server = Server::new(self.attributes_reader());
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

        ctx.start_worker(addr.clone(), Uppercase, AllowAll, AllowAll)
            .await?;

        self.registry
            .uppercase_services
            .insert(addr.clone(), Default::default());

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

        let maybe_trust_context_id = self.trust_context.as_ref().map(|c| c.id());
        let resource = Resource::assert_inline(addr.address());
        let ac = self
            .access_control(
                &resource,
                &actions::HANDLE_MESSAGE,
                maybe_trust_context_id,
                None,
            )
            .await?;

        WorkerBuilder::with_access_control(ac, Arc::new(AllowAll), addr.clone(), Echoer)
            .start(ctx)
            .await
            .map(|_| ())?;

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

    async fn build_access_control(
        &self,
        r: &Resource,
        a: &Action,
        project_id: &str,
        default: &Expr,
    ) -> Result<Arc<dyn IncomingAccessControl>> {
        // Populate environment with known attributes:
        let mut env = Env::new();
        env.put("resource.id", str(r.as_str()));
        env.put("action.id", str(a.as_str()));
        env.put("resource.project_id", str(project_id));
        // Check if a policy exists for (resource, action) and if not, then
        // create a default entry:
        if self.policies.get_policy(r, a).await?.is_none() {
            self.policies.set_policy(r, a, default).await?
        }
        Ok(Arc::new(PolicyAccessControl::new(
            self.policies.clone(),
            self.identities_repository(),
            r.clone(),
            a.clone(),
            env,
        )))
    }

    pub(super) async fn start_credential_issuer_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        project: String,
    ) -> Result<()> {
        if self.registry.authenticator_service.contains_key(&addr) {
            return Err(ApiError::generic(
                "Credential issuer service already started",
            ));
        }
        let action = actions::HANDLE_MESSAGE;
        let resource = Resource::new(&addr.to_string());
        let rule = eq([ident("resource.project_id"), ident("subject.project_id")]);
        let abac = self
            .build_access_control(&resource, &action, project.as_str(), &rule)
            .await?;
        let issuer = CredentialsIssuer::new(self.identities(), self.identifier(), project).await?;
        WorkerBuilder::with_access_control(abac, Arc::new(AllowAll), addr.clone(), issuer)
            .start(ctx)
            .await
            .map(|_| ())?;
        self.registry
            .authenticator_service
            .insert(addr, AuthenticatorServiceInfo::default());
        Ok(())
    }

    #[cfg(feature = "direct-authenticator")]
    pub(super) async fn start_direct_authenticator_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        project: String,
    ) -> Result<()> {
        if self.registry.authenticator_service.contains_key(&addr) {
            return Err(ApiError::generic(
                "Direct Authenticator  service already started",
            ));
        }
        let action = actions::HANDLE_MESSAGE;
        let resource = Resource::new(&addr.to_string());

        let abac = self
            .access_control(&resource, &action, Some(project.as_str()), None)
            .await?;

        let direct = crate::authenticator::direct::DirectAuthenticator::new(
            project.clone(),
            self.attributes_writer(),
        )
        .await?;

        WorkerBuilder::with_access_control(abac, Arc::new(AllowAll), addr.clone(), direct)
            .start(ctx)
            .await
            .map(|_| ())?;

        self.registry
            .authenticator_service
            .insert(addr, AuthenticatorServiceInfo::default());

        // TODO: remove this once compatibility with old clients is not required anymore
        let legacy_api = crate::authenticator::direct::LegacyApiConverter::new();
        ctx.start_worker("authenticator", legacy_api, AllowAll, AllowAll)
            .await?;

        Ok(())
    }

    pub(super) async fn start_enrollment_token_authenticator_pair(
        &mut self,
        ctx: &Context,
        issuer_addr: Address,
        acceptor_addr: Address,
        project: String,
    ) -> Result<()> {
        if self
            .registry
            .authenticator_service
            .contains_key(&issuer_addr)
            || self
                .registry
                .authenticator_service
                .contains_key(&acceptor_addr)
        {
            return Err(ApiError::generic(
                "Enrollment token Authenticator service already started",
            ));
        }
        let action = actions::HANDLE_MESSAGE;
        let resource = Resource::new(&issuer_addr.to_string());
        let (issuer, acceptor) = EnrollmentTokenAuthenticator::new_worker_pair(
            project.clone(),
            self.attributes_writer(),
        );
        let rule = and([
            eq([ident("resource.project_id"), ident("subject.project_id")]),
            eq([ident("subject.ockam-role"), str("enroller")]),
        ]);
        let abac = self
            .build_access_control(&resource, &action, project.as_str(), &rule)
            .await?;
        let allow_all = Arc::new(AllowAll);
        WorkerBuilder::with_access_control(abac, allow_all.clone(), issuer_addr.clone(), issuer)
            .start(ctx)
            .await
            .map(|_| ())?;
        WorkerBuilder::with_access_control(
            allow_all.clone(),
            allow_all,
            acceptor_addr.clone(),
            acceptor,
        )
        .start(ctx)
        .await
        .map(|_| ())?;
        self.registry
            .authenticator_service
            .insert(issuer_addr, AuthenticatorServiceInfo::default());
        self.registry
            .authenticator_service
            .insert(acceptor_addr, AuthenticatorServiceInfo::default());
        Ok(())
    }

    pub(super) async fn start_okta_identity_provider_service_impl(
        &mut self,
        ctx: &Context,
        addr: Address,
        tenant_base_url: &str,
        certificate: &str,
        attributes: &[&str],
        project: &str,
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
        let au = crate::okta::Server::new(
            self.attributes_writer(),
            project.to_string(),
            tenant_base_url,
            certificate,
            attributes,
        )?;
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
}

impl NodeManagerWorker {
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

    //TODO: split this into the different services it really starts
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
            let project = std::str::from_utf8(body.project()).unwrap();

            node_manager
                .start_direct_authenticator_service_impl(ctx, addr, project.to_string())
                .await?;

            node_manager
                .start_credential_issuer_service_impl(
                    ctx,
                    DefaultAddress::CREDENTIAL_ISSUER.into(),
                    project.to_string(),
                )
                .await?;
            node_manager
                .start_enrollment_token_authenticator_pair(
                    ctx,
                    DefaultAddress::ENROLLMENT_TOKEN_ISSUER.into(),
                    DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR.into(),
                    project.to_string(),
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
        let project = std::str::from_utf8(body.project()).unwrap();

        node_manager
            .start_okta_identity_provider_service_impl(
                ctx,
                addr,
                body.tenant_base_url(),
                body.certificate(),
                body.attributes(),
                project,
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

        let vs = crate::verifier::Verifier::new(node_manager.identities());
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
        ctx: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<ResponseBuilder> {
        let mut node_manager = self.node_manager.write().await;
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();
        let encoded_identity = body.public_identity();

        let decoded_identity =
            &hex::decode(encoded_identity).map_err(|_| ApiError::generic("Unable to decode trust context's public identity when starting credential service."))?;
        let i = identities()
            .identities_creation()
            .decode_identity(decoded_identity)
            .await?;

        let trust_context = TrustContext::new(
            encoded_identity.to_string(),
            Some(AuthorityService::new(
                node_manager.identities().identities_reader(),
                node_manager.credentials(),
                i.identifier(),
                None,
            )),
        );

        node_manager
            .start_credentials_service_impl(ctx, trust_context, addr, oneway)
            .await?;

        Ok(Response::ok(req.id()))
    }

    pub(super) async fn start_kafka_consumer_service<'a>(
        &mut self,
        context: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        // let mut node_manager = self.node_manager.write().await;
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

        self.start_kafka_service_impl(
            context,
            req,
            project_name,
            listener_address,
            body_req.bootstrap_server_addr.ip(),
            body_req.bootstrap_server_addr.port(),
            body_req.brokers_port_range(),
            project_route,
            KafkaServiceKind::Consumer,
        )
        .await?;

        Ok(Response::ok(req.id()).to_vec()?)
    }

    pub(super) async fn start_kafka_producer_service<'a>(
        &mut self,
        context: &Context,
        req: &'a Request<'_>,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        // let mut node_manager = self.node_manager.write().await;
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

        self.start_kafka_service_impl(
            context,
            req,
            project_name,
            listener_address,
            body_req.bootstrap_server_addr.ip(),
            body_req.bootstrap_server_addr.port(),
            body_req.brokers_port_range(),
            body_req.project_route().to_string().parse()?,
            KafkaServiceKind::Producer,
        )
        .await?;

        Ok(Response::ok(req.id()).to_vec()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn start_kafka_service_impl<'a>(
        &mut self,
        context: &Context,
        request: &'a Request<'_>,
        project_name: String,
        local_interceptor_address: Address,
        bind_ip: IpAddr,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        project_multiaddr: MultiAddr,
        kind: KafkaServiceKind,
    ) -> Result<()> {
        debug!("project_multiaddr: {}", project_multiaddr.to_string());

        let secure_channels = {
            // override default policy to allow incoming packets from the project
            let node_manager = self.node_manager.read().await;
            let (_addr, identity_identifier) = node_manager.resolve_project(&project_name)?;
            node_manager
                .policies
                .set_policy(
                    &resources::INLET,
                    &actions::HANDLE_MESSAGE,
                    &eq([
                        ident("subject.identifier"),
                        str(identity_identifier.to_string()),
                    ]),
                )
                .await?;

            node_manager.secure_channels.clone()
        };

        let secure_channel_controller =
            KafkaSecureChannelControllerImpl::new(secure_channels, project_multiaddr.clone());

        if let KafkaServiceKind::Consumer = kind {
            secure_channel_controller
                .create_consumer_listener(context)
                .await?;
        }

        let inlet_controller = KafkaInletController::new(
            project_multiaddr.clone(),
            route![local_interceptor_address.clone()],
            route![ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS],
            bind_ip,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::message("invalid port range"))?,
        );

        // since we cannot call APIs of node manager via message due to the read/write lock
        // we need to call it directly
        self.create_inlet_impl(
            request.id(),
            CreateInlet::to_node(
                SocketAddr::new(bind_ip, server_bootstrap_port),
                project_multiaddr,
                route![local_interceptor_address.clone()],
                route![
                    ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS,
                    ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS
                ],
                None,
            ),
            context,
        )
        .await?;

        KafkaPortalListener::create(
            context,
            inlet_controller,
            secure_channel_controller.into_trait(),
            local_interceptor_address.clone(),
        )
        .await?;

        {
            let mut node_manager = self.node_manager.write().await;
            node_manager
                .registry
                .kafka_services
                .insert(local_interceptor_address, KafkaServiceInfo::new(kind));
        }

        Ok(())
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
        registry
            .authenticator_service
            .keys()
            .for_each(|addr| list.push(ServiceStatus::new(addr.address(), "Authority")));

        Response::ok(req.id()).body(ServiceList::new(list))
    }
}
