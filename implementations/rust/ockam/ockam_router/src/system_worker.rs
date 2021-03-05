use crate::RouteTransportMessage;
use async_trait::async_trait;
use ockam::{Context, Worker};
use ockam_core::Result;

pub struct SystemWorker();

#[async_trait]
impl Worker for SystemWorker {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, _ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(_m) => {
                // The first byte of the payload is the message type.

                // If msg_type is ROUTER_MSG_REQUEST_CHANNEL, send a message to
                // CHANNEL_FACTORY_ADDRESS to create a responder.

                // I think we need a new message type for creating an initiator.

                Ok(())
            }
            _ => Ok(()),
        };
    }
}
