use super::NodeManagerWorker;
use crate::error::ApiError;
use crate::kafka::key_exchange::controller::KafkaKeyExchangeControllerImpl;
use crate::kafka::key_exchange::listener::KafkaKeyExchangeListener;
use crate::kafka::protocol_aware::inlet::KafkaInletInterceptorFactory;
use crate::kafka::protocol_aware::outlet::KafkaOutletInterceptorFactory;
use crate::kafka::KafkaOutletController;
use crate::kafka::{
    kafka_policy_expression, KafkaInletController, KAFKA_OUTLET_BOOTSTRAP_ADDRESS,
    KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
};
use crate::nodes::models::portal::OutletAccessControl;
use crate::nodes::models::services::{
    DeleteServiceRequest, StartKafkaCustodianRequest, StartKafkaInletRequest,
    StartKafkaOutletRequest, StartServiceRequest,
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
use ockam_core::compat::clock::ProductionClock;
use ockam_core::compat::rand::random_string;
use ockam_core::flow_control::FlowControls;
use ockam_core::route;
use ockam_multiaddr::proto::Project;
use ockam_transport_tcp::{PortalInletInterceptor, PortalOutletInterceptor};
use std::sync::Arc;

impl NodeManagerWorker {
    pub(super) async fn start_kafka_inlet_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaInletRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let request = body.request;
        match self
            .node_manager
            .start_kafka_inlet_service(context, Address::from_string(body.address), request)
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

    pub(crate) async fn start_custodian_service(
        &self,
        context: &Context,
        body: StartServiceRequest<StartKafkaCustodianRequest>,
    ) -> Result<Response<()>, Response<Error>> {
        let service_address = Address::from_string(body.address());
        let request = body.request;
        match self
            .node_manager
            .start_custodian_service(context, service_address, request)
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
    pub async fn start_kafka_inlet_service(
        &self,
        context: &Context,
        interceptor_address: Address,
        request: StartKafkaInletRequest,
    ) -> Result<()> {
        let consumer_policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(
                    format!("kafka-consumer-{}", interceptor_address.address()),
                    ResourceType::KafkaConsumer,
                ),
                Action::HandleMessage,
                request.consumer_policy_expression,
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
                request.producer_policy_expression,
            )
            .await?;

        let vault = if let Some(vault) = request.vault {
            let named_vault = self.cli_state.get_named_vault(&vault).await?;
            self.cli_state
                .make_vault(Some(context), named_vault)
                .await?
        } else {
            self.node_manager.secure_channels.vault().clone()
        };

        let key_exchange_controller = KafkaKeyExchangeControllerImpl::new(
            self.node_manager.clone(),
            vault.encryption_at_rest_vault.clone(),
            request.consumer_resolution,
            request.consumer_publishing,
            consumer_policy_access_control,
        );

        let inlet_policy_expression =
            if let Some(inlet_policy_expression) = request.inlet_policy_expression {
                Some(inlet_policy_expression)
            } else if let Some(project) = request
                .outlet_node_multiaddr
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
            request.outlet_node_multiaddr.clone(),
            route![interceptor_address.clone()],
            route![KAFKA_OUTLET_INTERCEPTOR_ADDRESS],
            request.bind_address.hostname(),
            PortRange::try_from(request.brokers_port_range)
                .map_err(|_| ApiError::core("invalid port range"))?,
            inlet_policy_expression.clone(),
        );

        // TODO: remove key exchange from inlets
        KafkaKeyExchangeListener::create(
            ProductionClock,
            context,
            vault.encryption_at_rest_vault,
            self.secure_channels.vault().secure_channel_vault,
            self.secure_channels.secure_channel_registry(),
            std::time::Duration::from_secs(60 * 60 * 24),
            std::time::Duration::from_secs(60 * 60 * 30),
            std::time::Duration::from_secs(60 * 60),
            producer_policy_access_control.create_incoming(),
            producer_policy_access_control
                .create_outgoing(context)
                .await?,
        )
        .await?;

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
            request.bind_address,
            route![interceptor_address.clone()],
            route![
                KAFKA_OUTLET_INTERCEPTOR_ADDRESS,
                KAFKA_OUTLET_BOOTSTRAP_ADDRESS
            ],
            request.outlet_node_multiaddr,
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
                key_exchange_controller,
                inlet_controller,
                request.encrypt_content,
                request.encrypted_fields,
            )),
            Arc::new(policy_access_control.create_incoming()),
            Arc::new(policy_access_control.create_outgoing(context).await?),
        )
        .await?;

        self.registry
            .kafka_services
            .insert(
                interceptor_address,
                KafkaServiceInfo::new(KafkaServiceKind::Inlet),
            )
            .await;

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
            Arc::new(policy_access_control.create_outgoing(context).await?),
            Arc::new(policy_access_control.create_incoming()),
        )
        .await?;

        // every secure channel can reach this service
        let flow_controls = context.flow_controls();
        flow_controls.add_consumer(
            interceptor_address.clone(),
            &default_secure_channel_listener_flow_control_id,
        );

        // this spawner flow control id is used to control communication with dynamically created
        // outlets
        flow_controls.add_spawner(interceptor_address.clone(), &spawner_flow_control_id);

        // allow communication with the kafka bootstrap outlet
        flow_controls.add_consumer(KAFKA_OUTLET_BOOTSTRAP_ADDRESS, &spawner_flow_control_id);

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

        self.registry
            .kafka_services
            .insert(
                service_address,
                KafkaServiceInfo::new(KafkaServiceKind::Outlet),
            )
            .await;

        Ok(())
    }

    pub async fn start_custodian_service(
        &self,
        context: &Context,
        service_address: Address,
        request: StartKafkaCustodianRequest,
    ) -> Result<()> {
        //TODO: dedup code with start_kafka_inlet_service
        let vault = if let Some(vault) = request.vault {
            let named_vault = self.cli_state.get_named_vault(&vault).await?;
            self.cli_state
                .make_vault(Some(context), named_vault)
                .await?
        } else {
            self.node_manager.secure_channels.vault().clone()
        };

        let producer_policy_access_control = self
            .policy_access_control(
                self.project_authority().clone(),
                Resource::new(
                    format!("kafka-producer-{}", service_address.address()),
                    ResourceType::KafkaProducer,
                ),
                Action::HandleMessage,
                request.producer_policy_expression,
            )
            .await?;

        KafkaKeyExchangeListener::create(
            ProductionClock,
            context,
            vault.encryption_at_rest_vault,
            self.secure_channels.vault().secure_channel_vault,
            self.secure_channels.secure_channel_registry(),
            request.key_rotation,
            request.key_validity,
            request.rekey_period,
            producer_policy_access_control.create_incoming(),
            producer_policy_access_control
                .create_outgoing(context)
                .await?,
        )
        .await?;

        self.registry
            .kafka_services
            .insert(
                DefaultAddress::KAFKA_CUSTODIAN.into(),
                KafkaServiceInfo::new(KafkaServiceKind::Custodian),
            )
            .await;

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
        match self.registry.kafka_services.get(&address).await {
            None => Ok(DeleteKafkaServiceResult::ServiceNotFound { address, kind }),
            Some(e) => {
                if kind.eq(e.kind()) {
                    match e.kind() {
                        KafkaServiceKind::Inlet => {
                            ctx.stop_worker(address.clone()).await?;
                        }
                        KafkaServiceKind::Outlet => {
                            ctx.stop_worker(KAFKA_OUTLET_INTERCEPTOR_ADDRESS).await?;
                            ctx.stop_worker(KAFKA_OUTLET_BOOTSTRAP_ADDRESS).await?;
                        }
                        KafkaServiceKind::Custodian => {
                            ctx.stop_worker(DefaultAddress::KAFKA_CUSTODIAN).await?;
                        }
                    }
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
