use crate::{PortalMessage, MAX_PAYLOAD_SIZE};
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{
    async_trait, route, Address, AllowOnwardAddress, AllowSourceAddress, Any,
    AnyIncomingAccessControl, AnyOutgoingAccessControl, Encodable, IncomingAccessControl,
    LocalInfo, LocalMessage, NeutralMessage, OutgoingAccessControl, Route, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, trace};

/// Direction of the data being intercepted
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    /// Data is moving from the inlet to the outlet
    FromOutletToInlet,
    /// Data is moving from the outlet to the inlet
    FromInletToOutlet,
}

/// Portal Interceptor
#[async_trait]
pub trait PortalInterceptor: 'static + Send + Sync {
    /// This method is called whenever a message is intercepted in either direction.
    /// The returned buffer can be of any size and will be sent to the original destination.
    /// The buffer will always be discarded after the call, and it's up to the interceptor
    /// to properly preserve it when needed.
    async fn intercept(
        &self,
        context: &mut Context,
        direction: Direction,
        buffer: &[u8],
    ) -> ockam_core::Result<Option<Vec<u8>>>;
}

/// Portal Interceptor Factory
pub trait PortalInterceptorFactory: 'static + Send + Sync {
    /// Create a new instance of a portal interceptor
    fn create(&self) -> Arc<dyn PortalInterceptor>;
}

/// Portal interceptor for the outlet side
pub struct PortalOutletInterceptor {
    interceptor_factory: Arc<dyn PortalInterceptorFactory>,
    outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    incoming_access_control: Arc<dyn IncomingAccessControl>,
    spawner_flow_control_id: Option<FlowControlId>,
}

impl PortalOutletInterceptor {
    /// Starts a listener that will intercept data in a portal on the outlet side.
    /// Every time a message is received, it'll spawn two workers to intercept the data.
    /// These two workers will replace the listener from the route, one for each direction.
    /// see [`PortalInterceptorWorker::create_outlet_interceptor`] for more details
    /// ```text
    /// ┌────────┐            ┌───────────┐             ┌────────┐
    /// │Secure  ├───────────►│ Listener  ├────────────►│TCP     │
    /// │Channel │            │           │             │Outlet  │
    /// └────────┘            └───────────┘             └────────┘
    /// ```
    pub async fn create(
        context: &Context,
        listener_address: Address,
        spawner_flow_control_id: Option<FlowControlId>,
        interceptor_factory: Arc<dyn PortalInterceptorFactory>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> ockam_core::Result<()> {
        let worker = Self {
            spawner_flow_control_id,
            interceptor_factory,
            outgoing_access_control,
            incoming_access_control: incoming_access_control.clone(),
        };

        WorkerBuilder::new(worker)
            .with_address(listener_address)
            .with_incoming_access_control_arc(incoming_access_control)
            .start(context)
            .await
            .map(|_| ())
    }
}

#[ockam_core::worker]
impl Worker for PortalOutletInterceptor {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let source_address = message.src_addr();
        let mut message = message.into_local_message();

        // Remove our address
        message = message.pop_front_onward_route()?;

        // unique flow control id for each interceptor instance
        let flow_control_id = FlowControls::generate_flow_control_id();

        let worker_address = PortalInterceptorWorker::create_outlet_interceptor(
            context,
            message.onward_route(),
            flow_control_id,
            self.spawner_flow_control_id.clone(),
            self.incoming_access_control.clone(),
            self.outgoing_access_control.clone(),
            self.interceptor_factory.create(),
        )
        .await?;

        // retrieve the flow id from the previous hop if it exists, usually a secure channel
        let source_flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(&source_address)
            .map(|x| x.flow_control_id().clone());

        if let Some(source_flow_control_id) = source_flow_control_id.as_ref() {
            // allows the source worker to communicate with the interceptor worker
            // which was just created
            context
                .flow_controls()
                .add_consumer(worker_address.clone(), source_flow_control_id);
        }

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

/// Portal Interceptor Listener
pub struct PortalInletInterceptor {
    interceptor_factory: Arc<dyn PortalInterceptorFactory>,
    request_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    response_incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl PortalInletInterceptor {
    /// Starts a listener that will intercept data in a portal on the inlet side.
    /// Every time a message is received, it'll spawn two workers to intercept the data.
    /// These two workers will replace the listener from the route, one for each direction.
    /// see [`PortalInterceptorWorker::create_inlet_interceptor`] for more details
    /// ```text
    /// ┌────────┐            ┌───────────┐             ┌────────┐
    /// │TCP     ├───────────►│ Listener  ├────────────►│Secure  │
    /// │Inlet   │            │           │             │Channel │
    /// └────────┘            └───────────┘             └────────┘
    /// ```
    pub async fn create(
        context: &Context,
        listener_address: Address,
        interceptor_factory: Arc<dyn PortalInterceptorFactory>,
        response_incoming_access_control: Arc<dyn IncomingAccessControl>,
        request_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> ockam_core::Result<()> {
        let worker = Self {
            interceptor_factory,
            request_outgoing_access_control,
            response_incoming_access_control,
        };

        context.start_worker(listener_address, worker).await
    }
}

#[ockam_core::worker]
impl Worker for PortalInletInterceptor {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        tracing::trace!("received message");

        let mut message = message.into_local_message();

        // Remove our address
        message = message.pop_front_onward_route()?;

        let next_hop = message.next_on_onward_route()?;

        // Per convention secure channel encryptor or transports
        // "outgoing" services, have an additional producer address
        // to retrieve the flow control "on the way back"

        // Retrieve the flow id from the next hop if it exists
        let flow_control_id = context
            .flow_controls()
            .find_flow_control_with_producer_address(&next_hop)
            .map(|x| x.flow_control_id().clone());

        let inlet_responder_address = message.return_route_ref().next()?.clone();

        let worker_address = PortalInterceptorWorker::create_inlet_interceptor(
            context,
            flow_control_id,
            route![inlet_responder_address],
            self.request_outgoing_access_control.clone(),
            self.response_incoming_access_control.clone(),
            self.interceptor_factory.create(),
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

/// Worker that intercepts data in a portal
pub struct PortalInterceptorWorker {
    fixed_onward_route: Option<Route>,
    other_worker_address: Address,
    disconnect_received: Arc<AtomicBool>,
    interceptor: Arc<dyn PortalInterceptor>,
    direction: Direction,
}

#[async_trait]
impl Worker for PortalInterceptorWorker {
    type Message = NeutralMessage;
    type Context = Context;

    async fn shutdown(&mut self, _context: &mut Context) -> ockam_core::Result<()> {
        //TODO: send disconnect to everyone?
        Ok(())
    }

    async fn handle_message(
        &mut self,
        context: &mut Context,
        routed_message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let onward_route = routed_message.onward_route();
        let return_route = routed_message.return_route();
        let local_info = routed_message.local_message().local_info();
        let portal_message = PortalMessage::decode(routed_message.payload())?;

        match portal_message {
            PortalMessage::Payload(message, _) => {
                let buffer: Option<Vec<u8>> = self
                    .interceptor
                    .intercept(context, self.direction, message)
                    .await?;
                match buffer {
                    Some(buffer) => {
                        trace!(
                            "buffer of size {} returned by the interceptor",
                            buffer.len()
                        );
                        self.split_and_send(
                            context,
                            onward_route.clone(),
                            return_route.clone(),
                            &buffer,
                            &local_info,
                        )
                        .await?;
                    }
                    None => {
                        trace!("empty buffer returned by the interceptor");
                    }
                }
            }
            PortalMessage::Disconnect => {
                self.forward(context, routed_message).await?;

                // the first one to receive disconnect and to swap the atomic will stop both workers
                let disconnect_received = self.disconnect_received.swap(true, Ordering::SeqCst);
                if !disconnect_received {
                    debug!(
                        "{:?} received disconnect event from {:?}",
                        context.address(),
                        return_route
                    );
                    context
                        .stop_worker(self.other_worker_address.clone())
                        .await?;
                    context.stop_worker(context.address()).await?;
                }
            }
            PortalMessage::Ping => self.forward(context, routed_message).await?,

            PortalMessage::Pong => {
                match self.direction {
                    Direction::FromInletToOutlet => {
                        // if we receive a pong message, it means it must be from the other worker
                        if routed_message.src_addr() == self.other_worker_address {
                            if let Some(fixed_onward_route) = self.fixed_onward_route.as_ref() {
                                debug!(
                                    "updating onward route from {} to {}",
                                    fixed_onward_route,
                                    routed_message.return_route()
                                );
                                self.fixed_onward_route = Some(routed_message.return_route());
                            }
                        }
                    }
                    Direction::FromOutletToInlet => {
                        // only the response worker should receive pongs but we forward
                        // the pong also to the other worker to update the fixed onward route
                        // with the final route
                        let mut local_message = routed_message.local_message().clone();
                        local_message = local_message
                            .set_onward_route(route![self.other_worker_address.clone()]);
                        context.forward(local_message).await?;

                        self.forward(context, routed_message).await?
                    }
                }
            }
        }

        Ok(())
    }
}

impl PortalInterceptorWorker {
    /// Creates two specular workers to intercept data next to the inlet.
    /// This topology usually is used in conjunction with a secure channel.
    /// ```text
    /// ┌────────┐            ┌───────────┐             ┌────────┐
    /// │        ├───────────►│Interceptor├────────────►│        │
    /// │        │            │ To Outlet │             │        │
    /// │TCP     │            └───────────┘             │Secure  │
    /// │Inlet   │            ┌───────────┐             │Channel │
    /// │        │            │Interceptor│             │        │
    /// │        │◄───────────│ To Inlet  │─────────────┤        │
    /// └────────┘            └───────────┘             └────────┘
    ///```
    ///
    /// - `outlet_route` is the route from the interceptor to the outlet.
    ///     This route is extracted from the first `Ping` message received.
    /// - `flow_control_id` flow control from the secure channel to the interceptor.
    /// - `inlet_instance` the route from the interceptor to the inlet.
    /// - `incoming_access_control` is the access control for the incoming messages.
    /// - `outgoing_access_control` is the access control for the outgoing messages.
    pub async fn create_inlet_interceptor(
        context: &mut Context,
        flow_control_id: Option<FlowControlId>,
        inlet_instance: Route,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        interceptor: Arc<dyn PortalInterceptor>,
    ) -> ockam_core::Result<Address> {
        let from_inlet_worker_address =
            Address::random_tagged("InterceptorPortalWorker.from_inlet_to_outlet");
        let from_outlet_worker_address =
            Address::random_tagged("InterceptorPortalWorker.from_outlet_to_inlet");
        let disconnect_received = Arc::new(AtomicBool::new(false));

        if let Some(flow_control_id) = flow_control_id {
            let flow_controls = context.flow_controls();
            flow_controls.add_consumer(from_outlet_worker_address.clone(), &flow_control_id);
        }

        let from_outlet_worker = Self {
            other_worker_address: from_inlet_worker_address.clone(),
            direction: Direction::FromOutletToInlet,
            disconnect_received: disconnect_received.clone(),
            fixed_onward_route: Some(inlet_instance),
            interceptor: interceptor.clone(),
        };

        WorkerBuilder::new(from_outlet_worker)
            .with_address(from_outlet_worker_address.clone())
            .with_incoming_access_control_arc(incoming_access_control)
            .start(context)
            .await?;

        let from_inlet_worker = Self {
            other_worker_address: from_outlet_worker_address,
            direction: Direction::FromInletToOutlet,
            disconnect_received: disconnect_received.clone(),
            fixed_onward_route: None,
            interceptor: interceptor.clone(),
        };

        WorkerBuilder::new(from_inlet_worker)
            .with_address(from_inlet_worker_address.clone())
            .with_outgoing_access_control_arc(outgoing_access_control)
            .start(context)
            .await?;

        Ok(from_inlet_worker_address)
    }

    /// Creates two specular workers to intercept data next to the outlet.
    /// This topology usually is used in conjunction with a secure channel.
    /// ```text
    /// ┌────────┐            ┌───────────┐             ┌────────┐
    /// │        ├───────────►│Interceptor├────────────►│        │
    /// │        │            │ To Outlet │             │        │
    /// │Secure  │            └───────────┘             │  TCP   │
    /// │Channel │            ┌───────────┐             │ Outlet │
    /// │        │            │Interceptor│             │        │
    /// │        │◄───────────│ To Inlet  │─────────────┤        │
    /// └────────┘            └───────────┘             └────────┘
    ///```
    ///
    /// - `outlet_route` is the route from the interceptor to the outlet.
    ///     This route is extracted from the first `Ping` message received.
    /// - `flow_control_id` new flow control id to control the communication with the outlet.
    /// - `spawner_flow_control_id` to account for future created outlets,
    /// - `incoming_access_control` is the access control for the incoming messages.
    /// - `outgoing_access_control` is the access control for the outgoing messages.
    async fn create_outlet_interceptor(
        context: &mut Context,
        outlet_route: Route,
        flow_control_id: FlowControlId,
        spawner_flow_control_id: Option<FlowControlId>,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        interceptor: Arc<dyn PortalInterceptor>,
    ) -> ockam_core::Result<Address> {
        let from_inlet_worker_address =
            Address::random_tagged("InterceptorPortalWorker.from_inlet_to_outlet");
        let from_outlet_worker_address =
            Address::random_tagged("InterceptorPortalWorker.from_outlet_to_inlet");
        let disconnect_received = Arc::new(AtomicBool::new(false));

        let from_inlet_worker = Self {
            other_worker_address: from_outlet_worker_address.clone(),
            direction: Direction::FromInletToOutlet,
            disconnect_received: disconnect_received.clone(),
            fixed_onward_route: Some(outlet_route),
            interceptor: interceptor.clone(),
        };
        let from_outlet_worker = Self {
            other_worker_address: from_inlet_worker_address.clone(),
            direction: Direction::FromOutletToInlet,
            disconnect_received: disconnect_received.clone(),
            fixed_onward_route: None,
            interceptor: interceptor.clone(),
        };

        let flow_controls = context.flow_controls();

        flow_controls.add_producer(
            from_inlet_worker_address.clone(),
            &flow_control_id,
            spawner_flow_control_id.as_ref(),
            vec![],
        );

        // allow the other worker to forward the `pong` message
        WorkerBuilder::new(from_inlet_worker)
            .with_address(from_inlet_worker_address.clone())
            .with_incoming_access_control_arc(Arc::new(AnyIncomingAccessControl::new(vec![
                Arc::new(AllowSourceAddress(from_outlet_worker_address.clone())),
                incoming_access_control,
            ])))
            .with_outgoing_access_control_arc(Arc::new(FlowControlOutgoingAccessControl::new(
                flow_controls,
                flow_control_id.clone(),
                spawner_flow_control_id.clone(),
            )))
            .start(context)
            .await?;

        // allow forwarding the `pong` message to the other worker
        let response_outgoing_access_control = {
            AnyOutgoingAccessControl::new(vec![
                Arc::new(AllowOnwardAddress::new(from_inlet_worker_address.clone())),
                outgoing_access_control,
            ])
        };

        WorkerBuilder::new(from_outlet_worker)
            .with_address(from_outlet_worker_address)
            .with_outgoing_access_control(response_outgoing_access_control)
            .start(context)
            .await?;

        Ok(from_inlet_worker_address)
    }

    async fn forward(
        &self,
        context: &mut Context,
        routed_message: Routed<NeutralMessage>,
    ) -> ockam_core::Result<()> {
        let mut local_message = routed_message.into_local_message();
        tracing::trace!(
            "before: onwards={:?}; return={:?};",
            local_message.onward_route_ref(),
            local_message.return_route_ref()
        );

        local_message = if let Some(fixed_onward_route) = &self.fixed_onward_route {
            tracing::trace!(
                "replacing onward_route {:?} with {:?}",
                local_message.onward_route_ref(),
                fixed_onward_route
            );
            local_message
                .set_onward_route(fixed_onward_route.clone())
                .push_front_return_route(&self.other_worker_address)
        } else {
            local_message = local_message.pop_front_onward_route()?;
            // Since we force the return route next step (fixed_onward_route in the other worker),
            // we can omit the previous return route.
            tracing::trace!(
                "replacing return_route {:?} with {:?}",
                local_message.return_route_ref(),
                self.other_worker_address
            );
            local_message.set_return_route(route![self.other_worker_address.clone()])
        };

        tracing::trace!(
            "after: onwards={:?}; return={:?};",
            local_message.onward_route_ref(),
            local_message.return_route_ref(),
        );
        context.forward(local_message).await
    }

    async fn split_and_send(
        &self,
        context: &mut Context,
        provided_onward_route: Route,
        provided_return_route: Route,
        buffer: &[u8],
        local_info: &[LocalInfo],
    ) -> ockam_core::Result<()> {
        let return_route: Route;
        let onward_route;

        if let Some(fixed_onward_route) = &self.fixed_onward_route {
            // To correctly proxy messages to the inlet or outlet side
            // we invert the return route when a message pass through
            return_route = provided_return_route
                .clone()
                .modify()
                .prepend(self.other_worker_address.clone())
                .into();
            onward_route = fixed_onward_route.clone();
        } else {
            // Since we force the return route next step (fixed_onward_route in the other worker),
            // we can omit the previous return route.
            return_route = route![self.other_worker_address.clone()];
            onward_route = provided_onward_route.clone().modify().pop_front().into();
        };

        for chunk in buffer.chunks(MAX_PAYLOAD_SIZE) {
            let message = LocalMessage::new()
                .with_onward_route(onward_route.clone())
                .with_return_route(return_route.clone())
                .with_payload(PortalMessage::Payload(chunk, None).encode()?)
                .with_local_info(local_info.to_vec());

            context.forward(message).await?;
        }
        Ok(())
    }
}
