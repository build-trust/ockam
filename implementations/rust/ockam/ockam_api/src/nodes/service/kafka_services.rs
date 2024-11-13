use super::NodeManagerWorker;
use crate::error::ApiError;
use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
use crate::kafka::protocol_aware::inlet::KafkaInletInterceptorFactory;
use crate::kafka::protocol_aware::outlet::KafkaOutletInterceptorFactory;
use crate::kafka::KafkaOutletController;
use crate::kafka::{
    kafka_policy_expression, ConsumerPublishing, ConsumerResolution, KafkaInletController,
    KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::models::services::{
    DeleteServiceRequest, StartKafkaInletRequest, StartKafkaOutletRequest, StartServiceRequest,
};
use crate::nodes::registry::{KafkaServiceInfo, KafkaServiceKind};
use crate::nodes::service::default_address::DefaultAddress;
use crate::nodes::InMemoryNode;
use crate::port_range::PortRange;
use ockam::transport::HostnamePort;
use ockam::{Address, Context, Result};
use ockam_abac::PolicyExpression;
use ockam_abac::{Action, Resource, ResourceType};
use ockam_core::api::{Error, Response};
use ockam_core::compat::rand::random_string;
use ockam_core::flow_control::FlowControls;
use ockam_core::route;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::MultiAddr;
use ockam_transport_tcp::{
    read_portal_payload_length, PortalInletInterceptor, PortalOutletInterceptor,
};
use std::sync::Arc;

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
                request.encrypted_fields(),
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
        interceptor_address: Address,
        bind_address: HostnamePort,
        brokers_port_range: (u16, u16),
        outlet_node_multiaddr: MultiAddr,
        encrypt_content: bool,
        encrypted_fields: Vec<String>,
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
                    format!("kafka-consumer-{}", interceptor_address.address()),
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
                    format!("kafka-producer-{}", interceptor_address.address()),
                    ResourceType::KafkaProducer,
                ),
                Action::HandleMessage,
                producer_policy_expression,
            )
            .await?;

        let secure_channel_controller = KafkaKeyExchangeControllerImpl::new(
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
            self.node_manager.clone(),
            outlet_node_multiaddr.clone(),
            route![interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            bind_address.hostname(),
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
            bind_address,
            route![interceptor_address.clone()],
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
            false,
            false,
            false,
            None,
        )
        .await?;

        let policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(interceptor_address.to_string(), ResourceType::TcpInlet),
                Action::HandleMessage,
                inlet_policy_expression,
            )
            .await?;

        PortalInletInterceptor::create(
            context,
            interceptor_address.clone(),
            Arc::new(KafkaInletInterceptorFactory::new(
                secure_channel_controller,
                inlet_controller,
                encrypt_content,
                encrypted_fields,
            )),
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(context)?),
            read_portal_payload_length(),
        )?;

        self.registry.kafka_services.insert(
            interceptor_address,
            KafkaServiceInfo::new(KafkaServiceKind::Inlet),
        );

        Ok(())
    }

    pub async fn start_kafka_outlet_service(
        &self,
        context: &Context,
        service_address: Address,
        bootstrap_server_addr: HostnamePort,
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

        let spawner_flow_control_id = FlowControls::generate_flow_control_id();
        let outlet_controller = KafkaOutletController::new(
            self.node_manager.clone(),
            outlet_policy_expression.clone(),
            tls,
        );
        let interceptor_address = Address::from_string(KAFKA_OUTLET_INTERCEPTOR_ADDRESS);

        PortalOutletInterceptor::create(
            context,
            interceptor_address.clone(),
            Some(spawner_flow_control_id.clone()),
            Arc::new(KafkaOutletInterceptorFactory::new(
                outlet_controller.clone(),
                spawner_flow_control_id.clone(),
            )),
            Arc::new(policy_access_control.create_outgoing(context)?),
            Arc::new(policy_access_control.create_incoming()),
            read_portal_payload_length(),
        )?;

        // every secure channel can reach this service
        let flow_controls = context.flow_controls();
        flow_controls.add_consumer(
            &interceptor_address,
            &default_secure_channel_listener_flow_control_id,
        );

        // this spawner flow control id is used to control communication with dynamically created
        // outlets
        flow_controls.add_spawner(&interceptor_address, &spawner_flow_control_id);

        // allow communication with the kafka bootstrap outlet
        flow_controls.add_consumer(
            &KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into(),
            &spawner_flow_control_id,
        );

        self.create_outlet(
            context,
            bootstrap_server_addr,
            tls,
            Some(KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into()),
            false,
            OutletAccessControl::WithPolicyExpression(outlet_policy_expression),
            false,
        )
        .await?;

        self.registry.kafka_services.insert(
            service_address,
            KafkaServiceInfo::new(KafkaServiceKind::Outlet),
        );

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
        debug!(address = %address, kind = %kind, "Deleting kafka service");
        match self.registry.kafka_services.get(&address) {
            None => Ok(DeleteKafkaServiceResult::ServiceNotFound { address, kind }),
            Some(e) => {
                if kind.eq(e.kind()) {
                    match e.kind() {
                        KafkaServiceKind::Inlet => {
                            ctx.stop_address(&address)?;
                        }
                        KafkaServiceKind::Outlet => {
                            ctx.stop_address(&KAFKA_OUTLET_INTERCEPTOR_ADDRESS.into())?;
                            ctx.stop_address(&KAFKA_OUTLET_BOOTSTRAP_ADDRESS.into())?;
                        }
                    }
                    self.registry.kafka_services.remove(&address);
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
