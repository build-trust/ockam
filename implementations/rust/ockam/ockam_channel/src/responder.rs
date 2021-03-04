use crate::ChannelError;
use async_trait::async_trait;
use ockam::{Address, Context, Result, Worker};
use ockam_key_exchange_core::KeyExchanger;
use ockam_key_exchange_xx::Responder;
use ockam_router::{
    RouteTransportMessage, RouteableAddress, TransportMessage, ROUTER_ADDRESS,
    ROUTER_MSG_REQUEST_CHANNEL,
};

pub struct XResponder {
    pub m_expected: u8,
    pub connection_address: Address,
    pub parent: Address,
    pub responder: Responder,
}

#[async_trait]
impl Worker for XResponder {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        self.m_expected = ROUTER_MSG_REQUEST_CHANNEL;
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(mut msg) => {
                return if !self.responder.is_complete() {
                    let m_received = msg.payload.remove(0);
                    if m_received != self.m_expected {
                        return Err(ChannelError::KeyExchange.into());
                    }
                    println!("XResponder processing: {}", m_received);
                    // discard any payload and get the next message
                    let _ = self.responder.process(&msg.payload)?;
                    let mut m = self.responder.process(&[])?;

                    let mut reply = TransportMessage::new();
                    reply.onward_route = msg.return_route.clone();
                    reply.return_address(RouteableAddress::Local(ctx.address().to_vec()));
                    m.insert(0, self.m_expected + 1);
                    self.m_expected += 2;
                    reply.payload = m;
                    ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(reply))
                        .await?;
                    if self.responder.is_complete() {
                        println!("XResponder exchange is complete");
                        // ToDo - apparently .finalize() takes 'self', not '&self'. And the borrow checker
                        // can't relinquish self.responder b/c it is behind a mutable reference. So
                        // the next line  doesn't compile.
                        // let key = self.responder.finalize()?;

                        // ToDo - work with Katharina to figure out how to pass the key
                        // back to the parent.
                        // ctx.send_message(
                        //     self.parent.clone(),
                        //     ExchangerMessage::ExchangeComplete(key),
                        // )
                        // .await
                        // .unwrap();
                    }
                    Ok(())
                } else {
                    Err(ChannelError::KeyExchange.into())
                };
            }
            _ => Ok(()),
        };
    }
}
