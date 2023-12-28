use std::net::IpAddr;

use ockam::{Address, Context, Result};
use ockam_abac::expr::{eq, ident, str};
use ockam_abac::Policy;
use ockam_core::api::{Error, Response};
use ockam_core::compat::net::SocketAddr;
use ockam_core::route;
use ockam_multiaddr::MultiAddr;

use super::{actions, resources, NodeManagerWorker};
use crate::error::ApiError;
use crate::kafka::{
    ConsumerNodeAddr, KafkaInletController, KafkaPortalListener, KafkaSecureChannelControllerImpl,
    KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::kafka::{OutletManagerService, PrefixRelayService};
use crate::nodes::models::services::{
    DeleteServiceRequest, StartKafkaConsumerRequest, StartKafkaDirectRequest,
    StartKafkaOutletRequest, StartKafkaProducerRequest, StartServiceRequest,
};
use crate::nodes::registry::{KafkaServiceInfo, KafkaServiceKind};
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::InMemoryNode;
use crate::nodes::NodeManager;
use crate::port_range::PortRange;

impl NodeManagerWorker {
    pub(super) async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaOutletRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        match self
            .node_manager
            .start_kafka_outlet_service(
                context,
                Address::from_string(body.address()),
                body.request().bootstrap_server_addr,
            )
            .await
        {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_kafka_direct_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaDirectRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        let consumer_route: Option<MultiAddr> =
            match request.consumer_route().map(|r| r.parse()).transpose() {
                Ok(multiaddr) => multiaddr,
                Err(e) => return Err(Response::bad_request_no_request(&e.to_string())),
            };

        match self
            .node_manager
            .start_kafka_direct_service(
                context,
                Address::from_string(body.address()),
                request.bind_address().ip(),
                request.bind_address().port(),
                request.brokers_port_range(),
                *request.bootstrap_server_addr(),
                consumer_route,
            )
            .await
        {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_kafka_consumer_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaConsumerRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        let outlet_node_multiaddr: MultiAddr = match request.project_route().to_string().parse() {
            Ok(multiaddr) => multiaddr,
            Err(e) => return Err(Response::bad_request_no_request(&e.to_string())),
        };

        match self
            .node_manager
            .start_kafka_service(
                context,
                Address::from_string(body.address()),
                request.bootstrap_server_addr().ip(),
                request.bootstrap_server_addr().port(),
                request.brokers_port_range(),
                outlet_node_multiaddr,
                KafkaServiceKind::Consumer,
            )
            .await
        {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_kafka_producer_service(
        &mut self,
        context: &Context,
        body: StartServiceRequest<StartKafkaProducerRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        let outlet_node_multiaddr: MultiAddr = match request.project_route().to_string().parse() {
            Ok(multiaddr) => multiaddr,
            Err(e) => return Err(Response::bad_request_no_request(&e.to_string())),
        };

        match self
            .node_manager
            .start_kafka_service(
                context,
                Address::from_string(body.address()),
                request.bootstrap_server_addr().ip(),
                request.bootstrap_server_addr().port(),
                request.brokers_port_range(),
                outlet_node_multiaddr,
                KafkaServiceKind::Producer,
            )
            .await
        {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(crate) async fn delete_kafka_service(
        &self,
        ctx: &Context,
        delete_service_request: DeleteServiceRequest,
        kind: KafkaServiceKind,
    ) -> Result<Response<()>, Response<Error>> {
        match self
            .node_manager
            .delete_kafka_service(ctx, delete_service_request.address(), kind)
            .await
        {
            Ok(DeleteKafkaServiceResult::ServiceDeleted) => Ok(Response::ok()),
            Ok(DeleteKafkaServiceResult::ServiceNotFound { address, kind }) => {
                Err(Response::not_found_no_request(
                    &format!("Service at address '{address}' with kind {kind} not found"),
                ))
            },
            Ok(DeleteKafkaServiceResult::IncorrectKind { address, actual, expected }) => {
                Err(Response::not_found_no_request(
                    &format!("Service at address '{address}' is not a kafka {expected}. A service of kind {actual} was found instead"),
                ))
            },
            Err(e) => Err(Response::internal_error_no_request( &e.to_string())),
        }
    }
}

impl InMemoryNode {
    #[allow(clippy::too_many_arguments)]
    pub async fn start_kafka_direct_service(
        &self,
        context: &Context,
        local_interceptor_address: Address,
        bind_ip: IpAddr,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        bootstrap_server_addr: SocketAddr,
        consumer_route: Option<MultiAddr>,
    ) -> Result<()> {
        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;

        {
            OutletManagerService::create(
                context,
                self.secure_channels.clone(),
                self.trust_context()?.id(),
                default_secure_channel_listener_flow_control_id,
            )
            .await?;
        }

        self.create_outlet(
            context,
            bootstrap_server_addr,
            KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into(),
            Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.to_string()),
            false,
            None,
        )
        .await?;

        let trust_context_id;
        let secure_channels;
        {
            trust_context_id = self.trust_context()?.id().to_string();
            secure_channels = self.secure_channels.clone();
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
        self.create_inlet(
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
            self.registry
                .kafka_services
                .insert(
                    local_interceptor_address,
                    KafkaServiceInfo::new(KafkaServiceKind::Direct),
                )
                .await;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn start_kafka_service(
        &self,
        context: &Context,
        local_interceptor_address: Address,
        bind_ip: IpAddr,
        server_bootstrap_port: u16,
        brokers_port_range: (u16, u16),
        outlet_node_multiaddr: MultiAddr,
        kind: KafkaServiceKind,
    ) -> Result<()> {
        debug!(
            "outlet_node_multiaddr: {}",
            outlet_node_multiaddr.to_string()
        );

        let trust_context_id;
        let secure_channels;
        {
            trust_context_id = self.trust_context()?.id().to_string();
            secure_channels = self.secure_channels.clone();

            if let Some(project) = outlet_node_multiaddr.first().and_then(|value| {
                value
                    .cast::<ockam_multiaddr::proto::Project>()
                    .map(|p| p.to_string())
            }) {
                let (_, project_identifier) = self.resolve_project(&project).await?;
                // if we are using the project we need to allow safe communication based on the
                // project identifier
                self.cli_state
                    .set_policy(
                        &resources::INLET,
                        &actions::HANDLE_MESSAGE,
                        &Policy::new(eq([ident("subject.identifier"), str(project_identifier)])),
                    )
                    .await
                    .map_err(ockam_core::Error::from)?
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
        self.create_inlet(
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
            self.registry
                .kafka_services
                .insert(local_interceptor_address, KafkaServiceInfo::new(kind))
                .await;
        }

        Ok(())
    }
}

impl NodeManager {
    pub async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        service_address: Address,
        bootstrap_server_addr: SocketAddr,
    ) -> Result<()> {
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
                self.secure_channels.clone(),
                self.trust_context()?.id(),
                default_secure_channel_listener_flow_control_id,
            )
            .await?;
        }

        if let Err(e) = self
            .create_outlet(
                context,
                bootstrap_server_addr,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into(),
                Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.to_string()),
                false,
                None,
            )
            .await
        {
            return Err(ApiError::core(e.to_string()));
        };

        {
            self.registry
                .kafka_services
                .insert(
                    service_address,
                    KafkaServiceInfo::new(KafkaServiceKind::Outlet),
                )
                .await;
        }

        Ok(())
    }

    /// Delete a Kafka service from the registry.
    /// The expected kind must match the actual kind
    pub async fn delete_kafka_service(
        &self,
        ctx: &Context,
        address: Address,
        kind: KafkaServiceKind,
    ) -> Result<DeleteKafkaServiceResult> {
        match self.registry.kafka_services.get(&address).await {
            None => Ok(DeleteKafkaServiceResult::ServiceNotFound { address, kind }),
            Some(e) => {
                if kind.eq(e.kind()) {
                    ctx.stop_worker(address.clone()).await?;
                    self.registry.kafka_services.remove(&address).await;
                    Ok(DeleteKafkaServiceResult::ServiceDeleted)
                } else {
                    error!(address = %address, "Service is not a kafka {}", kind.to_string());
                    Ok(DeleteKafkaServiceResult::IncorrectKind {
                        address,
                        actual: e.kind().clone(),
                        expected: kind,
                    })
                }
            }
        }
    }
}

pub enum DeleteKafkaServiceResult {
    ServiceDeleted,
    IncorrectKind {
        address: Address,
        actual: KafkaServiceKind,
        expected: KafkaServiceKind,
    },
    ServiceNotFound {
        address: Address,
        kind: KafkaServiceKind,
    },
}
