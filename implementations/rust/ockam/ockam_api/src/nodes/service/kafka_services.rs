use ockam::transport::HostnamePort;
use ockam::{Address, Context, Result};
use ockam_abac::PolicyExpression;
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::api::{Error, Response};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::rand::random_string;
use ockam_core::route;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use std::str::FromStr;
use std::sync::Arc;

use super::NodeManagerWorker;
use crate::error::ApiError;
use crate::kafka::secure_channel_map::controller::KafkaSecureChannelControllerImpl;
use crate::kafka::OutletManagerService;
use crate::kafka::{
    kafka_policy_expression, ConsumerPublishing, ConsumerResolution, KafkaInletController,
    KafkaPortalListener, KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::models::services::{
    DeleteServiceRequest, StartKafkaInletRequest, StartKafkaOutletRequest, StartServiceRequest,
};
use crate::nodes::registry::{KafkaServiceInfo, KafkaServiceKind};
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::InMemoryNode;
use crate::port_range::PortRange;

impl NodeManagerWorker {
    pub(super) async fn start_kafka_inlet_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaInletRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        match self
            .node_manager
            .start_kafka_inlet_service(
                context,
                Address::from_string(body.address()),
                request.bind_address(),
                request.brokers_port_range(),
                request.project_route(),
                request.encrypt_content(),
                request.consumer_resolution(),
                request.consumer_publishing(),
                request.inlet_policy_expression(),
                request.consumer_policy_expression(),
                request.producer_policy_expression(),
            )
            .await
        {
            Ok(_) => Ok(Response::ok().body(())),
            Err(e) => Err(Response::internal_error_no_request(&e.to_string())),
        }
    }

    pub(super) async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaOutletRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request();
        match self
            .node_manager
            .start_kafka_outlet_service(
                context,
                Address::from_string(body.address()),
                request.bootstrap_server_addr(),
                request.tls(),
                request.policy_expression(),
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
    pub async fn start_kafka_inlet_service(
        &self,
        context: &Context,
        local_interceptor_address: Address,
        bind_address: SocketAddr,
        brokers_port_range: (u16, u16),
        outlet_node_multiaddr: MultiAddr,
        encrypt_content: bool,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
        inlet_policy_expression: Option<PolicyExpression>,
        consumer_policy_expression: Option<PolicyExpression>,
        producer_policy_expression: Option<PolicyExpression>,
    ) -> Result<()> {
        let consumer_policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(
                    format!("kafka-consumer-{}", local_interceptor_address.address()),
                    ResourceType::KafkaConsumer,
                ),
                Action::HandleMessage,
                consumer_policy_expression,
            )
            .await?;

        let producer_policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(
                    format!("kafka-producer-{}", local_interceptor_address.address()),
                    ResourceType::KafkaProducer,
                ),
                Action::HandleMessage,
                producer_policy_expression,
            )
            .await?;

        let secure_channel_controller = KafkaSecureChannelControllerImpl::new(
            self.node_manager.clone(),
            self.secure_channels.clone(),
            consumer_resolution,
            consumer_publishing,
            consumer_policy_access_control,
            producer_policy_access_control,
        );

        self.node_manager
            .start_key_exchanger_service(context, DefaultAddress::KEY_EXCHANGER_LISTENER.into())
            .await?;

        let inlet_policy_expression = if let Some(inlet_policy_expression) = inlet_policy_expression
        {
            Some(inlet_policy_expression)
        } else if let Some(project) = outlet_node_multiaddr
            .first()
            .and_then(|v| v.cast::<Project>().map(|p| p.to_string()))
        {
            let (_, project_identifier) = self.resolve_project(&project).await?;
            Some(PolicyExpression::FullExpression(kafka_policy_expression(
                &project_identifier,
            )))
        } else {
            None
        };

        let inlet_controller = KafkaInletController::new(
            outlet_node_multiaddr.clone(),
            route![local_interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_address.ip(),
            PortRange::try_from(brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
            inlet_policy_expression.clone(),
        );

        // tldr: the alias for the inlet must be unique, and we want to keep it readable.
        // This function will create an inlet for either a producer or a consumer.
        // Since the policy is hardcoded (see the expression above) and it's the same
        // for both types of services, we could just share the policy. However, since the
        // alias must be unique amongst all the registered inlets, it must be unique to
        // allow the user to use multiple producers or consumers within the same node.
        // For that reason, we add a prefix based on the service kind to have better
        // readability and a random component at the end to keep it unique.
        let inlet_alias = format!("kafka-inlet-{}", random_string());

        // create the kafka bootstrap inlet
        self.create_inlet(
            context,
            bind_address.to_string(),
            route![local_interceptor_address.clone()],
            route![
                KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS
            ],
            outlet_node_multiaddr,
            inlet_alias,
            inlet_policy_expression.clone(),
            None,
            None,
            true,
            None,
        )
        .await?;

        let policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(
                    local_interceptor_address.to_string(),
                    ResourceType::TcpInlet,
                ),
                Action::HandleMessage,
                inlet_policy_expression,
            )
            .await?;

        KafkaPortalListener::create(
            context,
            encrypt_content,
            inlet_controller,
            secure_channel_controller,
            local_interceptor_address.clone(),
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(context).await?),
        )
        .await?;

        self.registry
            .kafka_services
            .insert(
                local_interceptor_address,
                KafkaServiceInfo::new(KafkaServiceKind::Inlet),
            )
            .await;

        Ok(())
    }

    pub async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        service_address: Address,
        bootstrap_server_addr: String,
        tls: bool,
        outlet_policy_expression: Option<PolicyExpression>,
    ) -> Result<()> {
        let default_secure_channel_listener_flow_control_id = context
            .flow_controls()
            .get_flow_control_with_spawner(&DefaultAddress::SECURE_CHANNEL_LISTENER.into())
            .ok_or_else(|| {
                ApiError::core("Unable to get flow control for secure channel listener")
            })?;

        let policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(service_address.to_string(), ResourceType::TcpOutlet),
                Action::HandleMessage,
                outlet_policy_expression.clone(),
            )
            .await?;

        OutletManagerService::create(
            context,
            default_secure_channel_listener_flow_control_id,
            outlet_policy_expression.clone(),
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(context).await?),
            tls,
        )
        .await?;

        if let Err(e) = self
            .create_outlet(
                context,
                HostnamePort::from_str(&bootstrap_server_addr)?,
                tls,
                Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into()),
                false,
                OutletAccessControl::WithPolicyExpression(outlet_policy_expression),
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
