use crate::kafka::outlet_controller::KafkaOutletController;
use crate::kafka::portal_worker::KafkaPortalWorker;
use crate::kafka::protocol_aware::OutletInterceptorImpl;
use crate::kafka::{KAFKA_OUTLET_BOOTSTRAP_ADDRESS, KAFKA_OUTLET_INTERCEPTOR_ADDRESS};
use ockam::identity::{SecureChannels, TRUST_CONTEXT_ID_UTF8};
use ockam::{Any, Context, Result, Routed, Worker};
use ockam_abac::AbacAccessControl;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::Address;
use ockam_node::WorkerBuilder;
use std::sync::Arc;

/// This service handles the central component which is responsible for creating connections
/// to the kafka cluster as well as act as a relay for consumers.
/// Normally this services is hosted by the Orchestrator (with a different implementation),
/// this implementation was created to allow local usage.
pub(crate) struct OutletManagerService {
    outlet_controller: KafkaOutletController,
    incoming_access_control: Arc<AbacAccessControl>,
    flow_control_id: FlowControlId,
    spawner_flow_control_id: FlowControlId,
    outgoing_access_control: Arc<FlowControlOutgoingAccessControl>,
}

impl OutletManagerService {
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        trust_context_id: &str,
        default_secure_channel_listener_flow_control_id: FlowControlId,
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

        // add the default outlet as consumer for the interceptor
        flow_controls.add_consumer(KAFKA_OUTLET_BOOTSTRAP_ADDRESS, &flow_control_id);

        let worker = OutletManagerService {
            outlet_controller: KafkaOutletController::new(),
            incoming_access_control: Arc::new(AbacAccessControl::create(
                secure_channels.identities().repository(),
                TRUST_CONTEXT_ID_UTF8,
                trust_context_id,
            )),
            flow_control_id: flow_control_id.clone(),
            outgoing_access_control: Arc::new(FlowControlOutgoingAccessControl::new(
                flow_controls,
                flow_control_id.clone(),
                Some(spawner_flow_control_id.clone()),
            )),
            spawner_flow_control_id,
        };

        let incoming = worker.incoming_access_control.clone();
        let outgoing = worker.outgoing_access_control.clone();
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
        message.transport_mut().onward_route.step()?;
        let onward_route = message.transport().onward_route.clone();

        // Retrieve the flow id from the previous hop if it exists
        let secure_channel_flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(&source_address)
            .map(|x| x.flow_control_id().clone());

        let worker_address = KafkaPortalWorker::create_outlet_side_kafka_portal(
            context,
            None,
            onward_route,
            Arc::new(OutletInterceptorImpl::new(
                self.outlet_controller.clone(),
                self.flow_control_id.clone(),
            )),
            &context.flow_controls().clone(),
            secure_channel_flow_control_id,
            Some(self.flow_control_id.clone()),
            Some(self.spawner_flow_control_id.clone()),
            self.incoming_access_control.clone(),
            self.outgoing_access_control.clone(),
        )
        .await?;

        message
            .transport_mut()
            .onward_route
            .modify()
            .prepend(worker_address.clone());

        trace!(
            "forwarding message: onward={:?}; return={:?}; worker={:?}",
            &message.transport().onward_route,
            &message.transport().return_route,
            worker_address
        );

        context.forward(message).await?;
        Ok(())
    }
}
