use ockam_core::compat::sync::Arc;
use tracing::trace;

use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::{Address, AllowAll, Any, Error, Route, Routed, Worker};
use ockam_node::Context;

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
    flow_controls: FlowControls,
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

        let secure_channel_address = {
            let onward: Route = message.onward_route().modify().pop_front().into();
            onward.next()?.clone()
        };

        //retrieve the flow id from the source address
        let flow_control_id;
        if let Some(producer_info) = self
            .flow_controls
            .find_flow_control_with_producer_address(&secure_channel_address)
        {
            flow_control_id = producer_info.flow_control_id().clone();
        } else {
            return Err(Error::new(
                Origin::Application,
                Kind::Invalid,
                format!(
                    "no flow control id found in worker address {}",
                    secure_channel_address.address()
                ),
            ));
        }

        let worker_address = KafkaPortalWorker::start_kafka_portal(
            context,
            self.secure_channel_controller.clone(),
            self.uuid_to_name.clone(),
            self.inlet_controller.clone(),
            None,
            Some(&(self.flow_controls.clone(), flow_control_id)),
        )
        .await?;

        //forward to the worker and place its address in the return route
        let mut message = message.into_local_message();

        message
            .transport_mut()
            .onward_route
            .modify()
            .replace(worker_address.clone());

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
        flow_control: FlowControls,
    ) -> ockam_core::Result<()> {
        context
            .start_worker(
                listener_address,
                Self {
                    inlet_controller,
                    secure_channel_controller,
                    uuid_to_name: Default::default(),
                    flow_controls: flow_control,
                },
                AllowAll,
                AllowAll,
            )
            .await
    }
}
