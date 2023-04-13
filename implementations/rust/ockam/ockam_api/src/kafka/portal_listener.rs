use ockam_core::compat::sync::Arc;
use ockam_core::{route, Address, AllowAll, Any, Routed, Worker};
use ockam_node::Context;
use tracing::trace;

use crate::kafka::inlet_controller::KafkaInletController;
use crate::kafka::portal_worker::KafkaPortalWorker;
use crate::kafka::protocol_aware::TopicUuidMap;
use crate::kafka::secure_channel_map::KafkaSecureChannelController;

///First point of ingress of kafka connections, at the first message it spawns new stateful workers
/// to take care of the connection.
pub(crate) struct KafkaPortalListener {
    inlet_controller: KafkaInletController,
    secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
    uuid_to_name: TopicUuidMap,
}

#[ockam::worker]
impl Worker for KafkaPortalListener {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam::Result<()> {
        trace!("received first message!");

        let mut message = message.into_local_message();

        // Remove our address
        message.transport_mut().onward_route.step()?;

        let next_hop = message.transport().onward_route.next()?;

        // Retrieve the flow id from the next hop if it exists
        let flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(next_hop)
            .map(|x| x.flow_control_id().clone());

        let inlet_responder_address = message.transport().return_route.next()?.clone();

        let worker_address = KafkaPortalWorker::start_kafka_portal(
            context,
            self.secure_channel_controller.clone(),
            self.uuid_to_name.clone(),
            self.inlet_controller.clone(),
            None,
            flow_control_id,
            route![inlet_responder_address],
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

impl KafkaPortalListener {
    pub(crate) async fn create(
        context: &Context,
        inlet_controller: KafkaInletController,
        secure_channel_controller: Arc<dyn KafkaSecureChannelController>,
        listener_address: Address,
    ) -> ockam_core::Result<()> {
        context
            .start_worker(
                listener_address,
                Self {
                    inlet_controller,
                    secure_channel_controller,
                    uuid_to_name: Default::default(),
                },
                AllowAll,
                AllowAll,
            )
            .await
    }
}
