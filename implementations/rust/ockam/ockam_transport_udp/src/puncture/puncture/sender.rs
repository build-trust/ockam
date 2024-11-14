use crate::puncture::puncture::message::PunctureMessage;
use crate::puncture::puncture::notification::{wait_for_puncture, UdpPunctureNotification};
use crate::PunctureError;
use ockam_core::{Any, Encodable, LocalMessage, Result, Route, Routed, Worker};
use ockam_node::Context;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tracing::trace;

/// Worker that forwards messages from our node to the other side of the puncture.
pub(crate) struct UdpPunctureSenderWorker {
    notify_puncture_open_receiver: Receiver<UdpPunctureNotification>,
    peer_route: Option<Route>,
}

impl UdpPunctureSenderWorker {
    pub fn new(notify_puncture_open_receiver: Receiver<UdpPunctureNotification>) -> Self {
        Self {
            notify_puncture_open_receiver,
            peer_route: None,
        }
    }

    async fn handle_local(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        trace!("UDP puncture forward: Local => Remote: {:?}", msg);

        let peer_route = self
            .peer_route
            .clone()
            .ok_or(PunctureError::PunctureNotOpen)?;

        let msg = msg.into_local_message();

        let onward_route = msg.onward_route.modify().pop_front().into();
        let return_route = msg.return_route;

        // Wrap payload
        let wrapped_payload = PunctureMessage::Payload {
            onward_route,
            return_route,
            payload: msg.payload,
        };

        let msg = LocalMessage::new()
            .with_onward_route(peer_route)
            .with_payload(wrapped_payload.encode()?);

        // Forward
        ctx.forward(msg).await
    }
}

#[ockam_core::worker]
impl Worker for UdpPunctureSenderWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        self.peer_route =
            Some(wait_for_puncture(&mut self.notify_puncture_open_receiver, Duration::MAX).await?);

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.handle_local(ctx, msg).await?;

        Ok(())
    }
}
