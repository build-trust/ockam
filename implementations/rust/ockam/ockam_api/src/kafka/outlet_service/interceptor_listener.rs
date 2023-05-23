use crate::kafka::outlet_controller::KafkaOutletController;
use crate::kafka::portal_worker::{KafkaPortalWorker, MAX_KAFKA_MESSAGE_SIZE};
use crate::kafka::protocol_aware::{KafkaMessageInterceptor, OutletInterceptorImpl};
use crate::kafka::{
    ORCHESTRATOR_KAFKA_BOOTSTRAP_ADDRESS, ORCHESTRATOR_KAFKA_CONSUMERS,
    ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS,
};
use crate::DefaultAddress;
use ockam::{Any, Context, Result, Routed, Worker};
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{route, Address, AllowAll, AllowOnwardAddresses};
use std::sync::Arc;

pub(crate) struct OutletManagerService {
    outlet_controller: KafkaOutletController,
}

impl OutletManagerService {
    pub(crate) async fn create(context: &Context) -> Result<()> {
        let worker = OutletManagerService {
            outlet_controller: KafkaOutletController::new(),
        };

        context
            .start_worker(
                Address::from_string(ORCHESTRATOR_KAFKA_INTERCEPTOR_ADDRESS),
                worker,
                AllowAll,
                AllowAll,
            )
            .await
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

        // Retrieve the flow id from the next hop if it exists
        let flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(&source_address)
            .map(|x| x.flow_control_id().clone());

        let worker_address = KafkaPortalWorker::create_outlet_side_kafka_portal(
            context,
            None,
            onward_route,
            Arc::new(OutletInterceptorImpl::new(self.outlet_controller.clone())),
            &context.flow_controls().clone(),
            flow_control_id,
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
