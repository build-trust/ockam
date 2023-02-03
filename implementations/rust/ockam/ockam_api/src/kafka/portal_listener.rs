use tracing::trace;

use ockam_core::{Address, AllowAll, Any, Route, Routed, Worker};
use ockam_node::Context;

use crate::kafka::inlet_map::KafkaInletMap;
use crate::kafka::portal_worker::KafkaPortalWorker;
use crate::port_range::PortRange;

///First point of ingress of kafka connections, at the first message it spawns new stateful workers
/// to take care of the connection.
pub(crate) struct KafkaPortalListener {
    inlet_map: KafkaInletMap,
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
        let worker_address = KafkaPortalWorker::start(context, self.inlet_map.clone()).await?;

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
    pub(crate) async fn start(
        context: &Context,
        interceptor_route: Route,
        listener_address: Address,
        bind_host: String,
        port_range: PortRange,
    ) -> ockam_core::Result<()> {
        let inlet_map = KafkaInletMap::new(interceptor_route, bind_host, port_range);

        context
            .start_worker(
                listener_address,
                Self {
                    inlet_map: inlet_map.clone(),
                },
                AllowAll,
                AllowAll,
            )
            .await
    }
}
