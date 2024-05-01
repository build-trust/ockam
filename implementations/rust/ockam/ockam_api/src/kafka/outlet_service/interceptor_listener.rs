use crate::kafka::outlet_controller::KafkaOutletController;
use crate::kafka::portal_worker::KafkaPortalWorker;
use crate::kafka::protocol_aware::OutletInterceptorImpl;
use crate::kafka::KAFKA_OUTLET_INTERCEPTOR_ADDRESS;
use ockam::identity::{Identifier, SecureChannels};
use ockam::{Any, Context, Result, Routed, Worker};
use ockam_abac::IncomingAbac;
use ockam_abac::PolicyExpression;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{Address, IncomingAccessControl};
use ockam_node::WorkerBuilder;
use std::sync::Arc;

/// This service handles the central component which is responsible for creating connections
/// to the kafka cluster as well as act as a relay for consumers.
/// Normally this services is hosted by the Orchestrator (with a different implementation),
/// this implementation was created to allow local usage.
pub(crate) struct OutletManagerService {
    outlet_controller: KafkaOutletController,
    incoming_access_control: Arc<dyn IncomingAccessControl>,
    spawner_flow_control_id: FlowControlId,
}

impl OutletManagerService {
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        authority_identifier: Identifier,
        default_secure_channel_listener_flow_control_id: FlowControlId,
        policy_expression: Option<PolicyExpression>,
    ) -> Result<()> {
        let flow_controls = context.flow_controls();

        let worker_address = Address::from_string(KAFKA_OUTLET_INTERCEPTOR_ADDRESS);
        flow_controls.add_consumer(
            worker_address.clone(),
            &default_secure_channel_listener_flow_control_id,
        );

        let flow_control_id = FlowControls::generate_flow_control_id();
        let spawner_flow_control_id = FlowControls::generate_flow_control_id();

        flow_controls.add_spawner(worker_address.clone(), &spawner_flow_control_id);

        let abac = IncomingAbac::check_credential_only(
            secure_channels.identities().identities_attributes(),
            authority_identifier,
        );
        // TOOD: Should we add outgoing?
        let worker = OutletManagerService {
            outlet_controller: KafkaOutletController::new(policy_expression),
            incoming_access_control: Arc::new(abac),
            spawner_flow_control_id: spawner_flow_control_id.clone(),
        };

        let incoming = worker.incoming_access_control.clone();
        let outgoing = Arc::new(FlowControlOutgoingAccessControl::new(
            flow_controls,
            flow_control_id.clone(),
            Some(spawner_flow_control_id),
        ));

        WorkerBuilder::new(worker)
            .with_address(worker_address)
            .with_incoming_access_control_arc(incoming)
            .with_outgoing_access_control_arc(outgoing)
            .start(context)
            .await
            .map(|_| ())
    }
}

#[ockam::worker]
impl Worker for OutletManagerService {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        let source_address = message.src_addr();
        let mut message = message.into_local_message();

        // Remove our address
        message = message.pop_front_onward_route()?;

        // Retrieve the flow id from the previous hop if it exists
        let secure_channel_flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(&source_address)
            .map(|x| x.flow_control_id().clone());

        let worker_address = KafkaPortalWorker::create_outlet_side_kafka_portal(
            context,
            None,
            message.onward_route(),
            Arc::new(OutletInterceptorImpl::new(
                self.outlet_controller.clone(),
                self.spawner_flow_control_id.clone(),
            )),
            &context.flow_controls().clone(),
            secure_channel_flow_control_id,
            Some(self.spawner_flow_control_id.clone()),
            self.incoming_access_control.clone(),
        )
        .await?;

        message = message.push_front_onward_route(&worker_address);

        trace!(
            "forwarding message: onward={:?}; return={:?}; worker={:?}",
            &message.onward_route_ref(),
            &message.return_route_ref(),
            worker_address
        );

        context.forward(message).await?;
        Ok(())
    }
}
