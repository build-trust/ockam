use std::net::IpAddr;

use crate::cli_state::random_name;
use ockam::{Address, Context, Result};
use ockam_core::api::{Error, Response};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::rand::random_string;
use ockam_core::route;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::HostnamePort;

use super::NodeManagerWorker;
use crate::error::ApiError;
use crate::kafka::{
    kafka_default_policy_expression, kafka_policy_expression, ConsumerNodeAddr,
    KafkaInletController, KafkaPortalListener, KafkaSecureChannelControllerImpl,
    KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::kafka::{OutletManagerService, PrefixRelayService};
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::models::services::{
    DeleteServiceRequest, StartKafkaDirectRequest, StartKafkaOutletRequest, StartKafkaRequest,
    StartServiceRequest,
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
        match self
            .node_manager
            .start_kafka_direct_service(
                context,
                Address::from_string(body.address()),
                request.bind_address().ip(),
                request.bind_address().port(),
                request.brokers_port_range(),
                *request.bootstrap_server_addr(),
                request.consumer_route(),
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
        body: StartServiceRequest<StartKafkaRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        match self
            .node_manager
            .start_kafka_service(
                context,
                Address::from_string(body.address()),
                request.bootstrap_server_addr().ip(),
                request.bootstrap_server_addr().port(),
                request.brokers_port_range(),
                request.project_route(),
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
        body: StartServiceRequest<StartKafkaRequest>,
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
            }
            Ok(DeleteKafkaServiceResult::IncorrectKind { address, actual, expected }) => {
                Err(Response::not_found_no_request(
                    &format!("Service at address '{address}' is not a kafka {expected}. A service of kind {actual} was found instead"),
                ))
            }
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
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

        let project_authority = self
            .project_authority
            .clone()
            .ok_or(ApiError::core("NodeManager has no authority"))?;

        let default_policy_expression = kafka_default_policy_expression();
        let outlet_policy_expression = match &consumer_route {
            None => Some(default_policy_expression),
            Some(r) => {
                if let Some(project) = r
                    .first()
                    .and_then(|v| v.cast::<Project>().map(|p| p.to_string()))
                {
                    let (_, project_identifier) = self.resolve_project(&project).await?;
                    Some(kafka_policy_expression(&project_identifier))
                } else {
                    Some(default_policy_expression)
                }
            }
        };

        OutletManagerService::create(
            context,
            self.secure_channels.clone(),
            project_authority.clone(),
            default_secure_channel_listener_flow_control_id,
            outlet_policy_expression.clone(),
        )
        .await?;
        self.create_outlet(
            context,
            HostnamePort::from_socket_addr(bootstrap_server_addr)?,
            false,
            Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into()),
            false,
            OutletAccessControl::PolicyExpression(outlet_policy_expression.clone()),
        )
        .await?;

        let secure_channels = self.secure_channels.clone();
        let consumer_node_addr = match consumer_route {
            Some(route) => ConsumerNodeAddr::Direct(route),
            None => ConsumerNodeAddr::None,
        };

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            secure_channels,
            consumer_node_addr,
            project_authority,
        );

        let inlet_controller = KafkaInletController::new(
            "/secure/api".parse()?,
            route![local_interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_ip,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
            outlet_policy_expression.clone(),
        );

        // since we cannot call APIs of node manager via message due to the read/write lock
        // we need to call it directly
        self.create_inlet(
            context,
            SocketAddr::new(bind_ip, server_bootstrap_port).to_string(),
            route![local_interceptor_address.clone()],
            route![
                KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS
            ],
            "/secure/api".parse()?,
            random_name(),
            outlet_policy_expression,
            None,
            None,
            true,
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

        let project_authority = self
            .project_authority
            .clone()
            .ok_or(ApiError::core("NodeManager has no authority"))?;

        let secure_channels = self.secure_channels.clone();
        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            secure_channels,
            ConsumerNodeAddr::Relay(outlet_node_multiaddr.clone()),
            project_authority,
        );

        let inlet_policy_expression = if let Some(project) = outlet_node_multiaddr
            .first()
            .and_then(|v| v.cast::<Project>().map(|p| p.to_string()))
        {
            let (_, project_identifier) = self.resolve_project(&project).await?;
            Some(kafka_policy_expression(&project_identifier))
        } else {
            Some(kafka_default_policy_expression())
        };

        let inlet_controller = KafkaInletController::new(
            outlet_node_multiaddr.clone(),
            route![local_interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_ip,
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
            inlet_policy_expression.clone(),
        );

        // tldr: the alias for the inlet must be unique and we want to keep it readable.
        // This function will create an inlet for either a producer or a consumer.
        // Since the policy is hardcoded (see the expression above) and it's the same
        // for both type of services, we could just share the policy. However, since the
        // alias must be unique amongst all the registered inlets, it must be unique to
        // allow the user to use multiple producers or consumers within the same node.
        // For that reason, we add a prefix based on the service kind to have better
        // readability and a random component at the end to keep it unique.
        let inlet_alias = format!("kafka-{}-inlet-{}", kind, random_string());

        // since we cannot call APIs of node manager via message due to the read/write lock
        // we need to call it directly
        self.create_inlet(
            context,
            SocketAddr::new(bind_ip, server_bootstrap_port).to_string(),
            route![local_interceptor_address.clone()],
            route![
                KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS
            ],
            outlet_node_multiaddr,
            inlet_alias,
            inlet_policy_expression,
            None,
            None,
            true,
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

        let project_authority = self
            .project_authority
            .clone()
            .ok_or(ApiError::core("NodeManager has no authority"))?;
        let outlet_policy_expression = None;

        OutletManagerService::create(
            context,
            self.secure_channels.clone(),
            project_authority,
            default_secure_channel_listener_flow_control_id,
            outlet_policy_expression.clone(),
        )
        .await?;

        if let Err(e) = self
            .create_outlet(
                context,
                HostnamePort::from_socket_addr(bootstrap_server_addr)?,
                false,
                Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into()),
                false,
                OutletAccessControl::PolicyExpression(outlet_policy_expression),
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
