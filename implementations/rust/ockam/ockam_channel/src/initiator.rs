use crate::ChannelError;
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_key_exchange_core::KeyExchanger;
use ockam_key_exchange_xx::Initiator;
use ockam_router::message::RouteableAddress;
use ockam_router::{
    Route, RouteTransportMessage, RouterAddress, TransportMessage, ROUTER_ADDRESS,
    ROUTER_ADDRESS_TYPE_LOCAL, ROUTER_MSG_M2, ROUTER_MSG_REQUEST_CHANNEL,
};

pub struct XInitiator {
    pub m_expected: u8,
    pub connection_address: Address,
    pub parent: Address,
    pub initiator: Initiator,
    pub route: Route,
}

#[async_trait]
impl Worker for XInitiator {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let mut m1 = TransportMessage::new();

        m1.payload = self.initiator.process(&[])?;
        m1.payload.insert(0, ROUTER_MSG_REQUEST_CHANNEL);
        m1.onward_route = self.route.clone();
        m1.return_route.addrs.insert(
            0,
            RouterAddress {
                address_type: ROUTER_ADDRESS_TYPE_LOCAL,
                address: ctx.address().to_vec(),
            },
        );
        self.m_expected = ROUTER_MSG_M2;
        ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(m1))
            .await?;
        println!("XInitiator sent m1");
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(mut msg) => {
                return if !self.initiator.is_complete() {
                    let m_received = msg.payload.remove(0);
                    println!("XInitiator processing: {}", m_received);
                    if m_received != self.m_expected {
                        return Err(ChannelError::KeyExchange.into());
                    }
                    // discard any payload and get the next message
                    let _ = self.initiator.process(&msg.payload)?;
                    let mut m = self.initiator.process(&[])?;
                    let key_complete = self.initiator.is_complete();

                    let mut reply = TransportMessage::new();
                    reply.onward_route = msg.return_route.clone();
                    reply.return_address(RouteableAddress::Local(ctx.address().to_vec()));
                    m.insert(0, self.m_expected + 1);
                    self.m_expected += 2;
                    reply.payload = m;
                    ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(reply))
                        .await?;
                    if key_complete {
                        println!("XInitiator exchange is complete");
                        // ToDo - apparently .finalize() takes 'self', not '&self'. And the borrow checker
                        // can't relinquish self.responder b/c it is behind a mutable reference. So
                        // the next line  doesn't compile.
                        // let key = self.initiator.finalize()?;

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
                    // ToDo - this should never happen, but I expect there's a bug in the
                    // node message router. Sometimes messages get delivered twice. Then this happens. And
                    // returning the error causes relay.rs to blow up because it calls unwrap on the result.
                    // So for now we just ignore it.
                    //Err(ChannelError::KeyExchange.into())
                    Ok(())
                };
            }
            _ => Ok(()),
        };
    }
}
