use std::net::IpAddr;

use minicbor::Decoder;

use ockam::identity::{identities, AuthorityService, TrustContext};
use ockam::{Address, Context, Result};
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::Resource;
use ockam_core::api::{Error, RequestHeader, Response};
use ockam_core::compat::net::SocketAddr;
use ockam_core::route;
use ockam_multiaddr::MultiAddr;
use ockam_node::WorkerBuilder;

use crate::auth::Server;
use crate::echoer::Echoer;
use crate::error::ApiError;
use crate::hop::Hop;
use crate::kafka::{
    ConsumerNodeAddr, KafkaInletController, KafkaPortalListener, KafkaSecureChannelControllerImpl,
    KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::kafka::{OutletManagerService, PrefixRelayService};
use crate::nodes::models::services::{
    DeleteServiceRequest, ServiceList, ServiceStatus, StartAuthenticatedServiceRequest,
    StartCredentialsService, StartEchoerServiceRequest, StartHopServiceRequest,
    StartKafkaConsumerRequest, StartKafkaDirectRequest, StartKafkaOutletRequest,
    StartKafkaProducerRequest, StartServiceRequest, StartUppercaseServiceRequest,
};
use crate::nodes::registry::{
    CredentialsServiceInfo, KafkaServiceInfo, KafkaServiceKind, Registry,
};
use crate::nodes::NodeManager;
use crate::port_range::PortRange;
use crate::uppercase::Uppercase;
use crate::DefaultAddress;
use crate::{actions, resources};

use super::NodeManagerWorker;

impl NodeManager {
    pub(super) async fn start_credentials_service_impl<'a>(
        &self,
        ctx: &Context,
        trust_context: TrustContext,
        addr: Address,
        oneway: bool,
    ) -> Result<()> {
        if self.registry.credentials_services.contains_key(&addr).await {
            return Err(ApiError::core("Credentials service exists at this address"));
        }

        self.credentials_service()
            .start(
                ctx,
                trust_context,
                self.identifier().clone(),
                addr.clone(),
                !oneway,
            )
            .await?;

        self.registry
            .credentials_services
            .insert(addr.clone(), CredentialsServiceInfo::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_authenticated_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self
            .registry
            .authenticated_services
            .contains_key(&addr)
            .await
        {
            return Err(ApiError::core(
                "Authenticated service exists at this address",
            ));
        }

        let server = Server::new(self.attributes_reader());
        ctx.start_worker(addr.clone(), server).await?;

        self.registry
            .authenticated_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_uppercase_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.uppercase_services.contains_key(&addr).await {
            return Err(ApiError::core("Uppercase service exists at this address"));
        }

        ctx.start_worker(addr.clone(), Uppercase).await?;

        self.registry
            .uppercase_services
            .insert(addr.clone(), Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_echoer_service_impl(
        &self,
        ctx: &Context,
        addr: Address,
    ) -> Result<()> {
        if self.registry.echoer_services.contains_key(&addr).await {
            return Err(ApiError::core("Echoer service exists at this address"));
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

        WorkerBuilder::new(Echoer)
            .with_address(addr.clone())
            .with_incoming_access_control_arc(ac)
            .start(ctx)
            .await?;

        self.registry
            .echoer_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }

    pub(super) async fn start_hop_service_impl(&self, ctx: &Context, addr: Address) -> Result<()> {
        if self.registry.hop_services.contains_key(&addr).await {
            return Err(ApiError::core("Hop service exists at this address"));
        }

        ctx.flow_controls()
            .add_consumer(addr.clone(), &self.api_transport_flow_control_id);

        ctx.start_worker(addr.clone(), Hop).await?;

        self.registry
            .hop_services
            .insert(addr, Default::default())
            .await;

        Ok(())
    }
}

impl NodeManagerWorker {
    pub(super) async fn start_authenticated_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartAuthenticatedServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_authenticated_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_uppercase_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartUppercaseServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_uppercase_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_echoer_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartEchoerServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager
            .start_echoer_service_impl(ctx, addr)
            .await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_hop_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let req_body: StartHopServiceRequest = dec.decode()?;
        let addr = req_body.addr.to_string().into();
        self.node_manager.start_hop_service_impl(ctx, addr).await?;
        Ok(Response::ok(req))
    }

    pub(super) async fn start_credentials_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Response, Response<Error>> {
        let body: StartCredentialsService = dec.decode()?;
        let addr: Address = body.address().into();
        let oneway = body.oneway();
        let encoded_identity = body.public_identity();

        let decoded_identity =
            &hex::decode(encoded_identity).map_err(|_| ApiError::core("Unable to decode trust context's public identity when starting credential service."))?;
        let i = identities()
            .identities_creation()
            .import(None, decoded_identity)
            .await?;

        let trust_context = TrustContext::new(
            encoded_identity.to_string(),
            Some(AuthorityService::new(
                self.node_manager.identities().credentials(),
                i.identifier().clone(),
                None,
            )),
        );

        self.node_manager
            .start_credentials_service_impl(ctx, trust_context, addr, oneway)
            .await?;

        Ok(Response::ok(req))
    }
    pub(super) async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        request: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: StartServiceRequest<StartKafkaOutletRequest> = dec.decode()?;

        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;

        PrefixRelayService::create(
            context,
            default_secure_channel_listener_flow_control_id.clone(),
        )
        .await?;

        {
            OutletManagerService::create(
                context,
                self.node_manager.secure_channels.clone(),
                self.node_manager.trust_context()?.id(),
                default_secure_channel_listener_flow_control_id,
            )
            .await?;
        }

        if let Err(e) = self
            .node_manager
            .create_outlet(
                context,
                body.request().bootstrap_server_addr,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into(),
                Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.to_string()),
                false,
            )
            .await
        {
            return Ok(e.to_string().into_bytes());
        };

        {
            self.node_manager
                .registry
                .kafka_services
                .insert(
                    body.address().into(),
                    KafkaServiceInfo::new(KafkaServiceKind::Outlet),
                )
                .await;
        }

        Ok(Response::ok(request).to_vec()?)
    }

    pub(super) async fn start_kafka_direct_service(
        &self,
        context: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: StartServiceRequest<StartKafkaDirectRequest> = dec.decode()?;
        let listener_address: Address = body.address().into();
        let body_req = body.request();

        let consumer_route: Option<MultiAddr> =
            if let Some(consumer_route) = body_req.consumer_route() {
                Some(consumer_route.parse()?)
            } else {
                None
            };

        if let Err(e) = self
            .start_direct_kafka_service_impl(
                context,
                listener_address,
                body_req.bind_address().ip(),
                body_req.bind_address().port(),
                body_req.brokers_port_range(),
                *body_req.bootstrap_server_addr(),
                consumer_route,
            )
            .await
        {
            return Ok(e.to_vec()?);
        };

        Ok(Response::ok(req).to_vec()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn start_direct_kafka_service_impl(
        &self,
        context: &Context,
        local_interceptor_address: Address,
        bind_ip: IpAddr,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        bootstrap_server_addr: SocketAddr,
        consumer_route: Option<MultiAddr>,
    ) -> Result<(), Response<Error>> {
        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;

        {
            OutletManagerService::create(
                context,
                self.node_manager.secure_channels.clone(),
                self.node_manager.trust_context()?.id(),
                default_secure_channel_listener_flow_control_id,
            )
            .await?;
        }

        self.node_manager
            .create_outlet(
                context,
                bootstrap_server_addr,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into(),
                Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.to_string()),
                false,
            )
            .await?;

        let trust_context_id;
        let secure_channels;
        {
            trust_context_id = self.node_manager.trust_context()?.id().to_string();
            secure_channels = self.node_manager.secure_channels.clone();
        }

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            secure_channels,
            ConsumerNodeAddr::Direct(consumer_route.clone()),
            trust_context_id,
        );

        let inlet_controller = KafkaInletController::new(
            "/secure/api".parse().unwrap(),
            route![local_interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_ip,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
        );

        // since we cannot call APIs of node manager via message due to the read/write lock
        // we need to call it directly
        self.node_manager
            .create_inlet(
                context,
                SocketAddr::new(bind_ip, server_bootstrap_port).to_string(),
                None,
                route![local_interceptor_address.clone()],
                route![
                    KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                    KAFKA_OUTLET_BOOTSTRAP_ADDRESS
                ],
                "/secure/api".parse().unwrap(),
                None,
                None,
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
            self.node_manager
                .registry
                .kafka_services
                .insert(
                    local_interceptor_address,
                    KafkaServiceInfo::new(KafkaServiceKind::Direct),
                )
                .await;
        }

        Ok(())
    }

    pub(super) async fn start_kafka_consumer_service(
        &self,
        context: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: StartServiceRequest<StartKafkaConsumerRequest> = dec.decode()?;
        let listener_address: Address = body.address().into();
        let body_req = body.request();
        let outlet_node_multiaddr = body_req.project_route().to_string().parse()?;

        if let Err(e) = self
            .start_kafka_service_impl(
                context,
                listener_address,
                body_req.bootstrap_server_addr.ip(),
                body_req.bootstrap_server_addr.port(),
                body_req.brokers_port_range(),
                outlet_node_multiaddr,
                KafkaServiceKind::Consumer,
            )
            .await
        {
            return Ok(e.to_vec()?);
        };

        Ok(Response::ok(req).to_vec()?)
    }

    pub(super) async fn start_kafka_producer_service(
        &mut self,
        context: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
    ) -> Result<Vec<u8>> {
        let body: StartServiceRequest<StartKafkaProducerRequest> = dec.decode()?;
        let listener_address: Address = body.address().into();
        let body_req = body.request();
        let outlet_node_multiaddr = body_req.project_route().to_string().parse()?;

        if let Err(e) = self
            .start_kafka_service_impl(
                context,
                listener_address,
                body_req.bootstrap_server_addr.ip(),
                body_req.bootstrap_server_addr.port(),
                body_req.brokers_port_range(),
                outlet_node_multiaddr,
                KafkaServiceKind::Producer,
            )
            .await
        {
            return Ok(e.to_vec()?);
        };

        Ok(Response::ok(req).to_vec()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn start_kafka_service_impl(
        &self,
        context: &Context,
        local_interceptor_address: Address,
        bind_ip: IpAddr,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        outlet_node_multiaddr: MultiAddr,
        kind: KafkaServiceKind,
    ) -> Result<(), Response<Error>> {
        debug!(
            "outlet_node_multiaddr: {}",
            outlet_node_multiaddr.to_string()
        );

        let trust_context_id;
        let secure_channels;
        {
            trust_context_id = self.node_manager.trust_context()?.id().to_string();
            secure_channels = self.node_manager.secure_channels.clone();

            if let Some(project) = outlet_node_multiaddr.first().and_then(|value| {
                value
                    .cast::<ockam_multiaddr::proto::Project>()
                    .map(|p| p.to_string())
            }) {
                let (_, project_identifier) = self.node_manager.resolve_project(&project).await?;
                // if we are using the project we need to allow safe communication based on the
                // project identifier
                self.node_manager
                    .policies
                    .set_policy(
                        &resources::INLET,
                        &actions::HANDLE_MESSAGE,
                        &eq([ident("subject.identifier"), str(project_identifier)]),
                    )
                    .await?;
            }
        }

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            secure_channels,
            ConsumerNodeAddr::Relay(outlet_node_multiaddr.clone()),
            trust_context_id,
        );

        let inlet_controller = KafkaInletController::new(
            outlet_node_multiaddr.clone(),
            route![local_interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_ip,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
        );

        // since we cannot call APIs of node manager via message due to the read/write lock
        // we need to call it directly
        self.node_manager
            .create_inlet(
                context,
                SocketAddr::new(bind_ip, server_bootstrap_port).to_string(),
                None,
                route![local_interceptor_address.clone()],
                route![
                    KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                    KAFKA_OUTLET_BOOTSTRAP_ADDRESS
                ],
                outlet_node_multiaddr,
                None,
                None,
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
            self.node_manager
                .registry
                .kafka_services
                .insert(local_interceptor_address, KafkaServiceInfo::new(kind))
                .await;
        }

        Ok(())
    }

    pub(crate) async fn delete_kafka_service(
        &self,
        ctx: &Context,
        req: &RequestHeader,
        dec: &mut Decoder<'_>,
        kind: KafkaServiceKind,
    ) -> Result<Response, Response<Error>> {
        let body: DeleteServiceRequest = match dec.decode() {
            Ok(it) => it,
            Err(err) => {
                return Err(Response::bad_request(req, &err.to_string()));
            }
        };
        let address = body.address();
        let res = match self
            .node_manager
            .registry
            .kafka_services
            .get(&address)
            .await
        {
            None => {
                return Err(Response::not_found(
                    req,
                    &format!("Service at address '{}' not found", address),
                ));
            }
            Some(e) => {
                if kind.eq(e.kind()) {
                    ctx.stop_worker(address.clone()).await?;
                    self.node_manager
                        .registry
                        .kafka_services
                        .remove(&address)
                        .await;
                    Response::ok(req)
                } else {
                    error!(address = %address, "Service is not a kafka {}", kind.to_string());
                    return Err(Response::internal_error(
                        req,
                        &format!("Service at address '{}' is not a kafka {}", address, kind),
                    ));
                }
            }
        };
        Ok(res)
    }

    pub(super) async fn list_services_of_type(
        &self,
        req: &RequestHeader,
        service_type: &str,
    ) -> Result<Vec<u8>> {
        if !DefaultAddress::is_valid(service_type) {
            return Ok(Response::bad_request(
                req,
                &format!("Service type '{service_type}' doesn't exist"),
            )
            .to_vec()?);
        }
        let services = Self::list_services_impl(&self.node_manager.registry).await;
        let filtered = services
            .into_iter()
            .filter(|service| service.service_type == service_type)
            .collect();
        Ok(Response::ok(req)
            .body(ServiceList::new(filtered))
            .to_vec()?)
    }

    pub(super) async fn list_services(&self, req: &RequestHeader) -> Result<Vec<u8>> {
        let services = Self::list_services_impl(&self.node_manager.registry).await;
        Ok(Response::ok(req)
            .body(ServiceList::new(services))
            .to_vec()?)
    }

    async fn list_services_impl(registry: &Registry) -> Vec<ServiceStatus> {
        let mut list = Vec::new();
        registry
            .authenticated_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::AUTHENTICATED_SERVICE,
                ))
            });
        registry
            .uppercase_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::UPPERCASE_SERVICE,
                ))
            });
        registry
            .echoer_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::ECHO_SERVICE,
                ))
            });
        registry.hop_services.keys().await.iter().for_each(|addr| {
            list.push(ServiceStatus::new(
                addr.address(),
                DefaultAddress::HOP_SERVICE,
            ))
        });
        registry
            .credentials_services
            .keys()
            .await
            .iter()
            .for_each(|addr| {
                list.push(ServiceStatus::new(
                    addr.address(),
                    DefaultAddress::CREDENTIALS_SERVICE,
                ))
            });
        registry
            .kafka_services
            .entries()
            .await
            .iter()
            .for_each(|(address, info)| {
                list.push(ServiceStatus::new(
                    address.address(),
                    match info.kind() {
                        KafkaServiceKind::Consumer => DefaultAddress::KAFKA_CONSUMER,
                        KafkaServiceKind::Producer => DefaultAddress::KAFKA_PRODUCER,
                        KafkaServiceKind::Outlet => DefaultAddress::KAFKA_OUTLET,
                        KafkaServiceKind::Direct => DefaultAddress::KAFKA_DIRECT,
                    },
                ))
            });

        list
    }
}
