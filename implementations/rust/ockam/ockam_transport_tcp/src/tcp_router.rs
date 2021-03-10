use crate::{TcpWorkerMessage, TransportError};
use ockam::{Address, Context, Routed, Worker};
use ockam_core::async_trait::async_trait;
use ockam_core::Result;
use ockam_router::router::RouteTransportMessage;
use ockam_router::{RouterAddress, RouterError};
use std::collections::HashMap;

pub const TCP_ROUTER_ADDRESS: &str = "tcp_router";

pub struct TcpMessageRouter {
    registry: HashMap<Vec<u8>, Address>, // <vectorized sockeaddr, worker address>,
}

impl TcpMessageRouter {
    pub fn new() -> Self {
        TcpMessageRouter {
            registry: HashMap::new(),
        }
    }
    pub fn register(&mut self, addr: Address) -> Result<()> {
        let key = addr.to_vec();
        if self.registry.contains_key(&key.clone()) {
            return Err(RouterError::KeyInUse.into());
        }
        if self.registry.insert(key.clone(), addr).is_some() {
            return Err(RouterError::Stop.into());
        }
        Ok(())
    }
}

#[async_trait]
impl Worker for TcpMessageRouter {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg = msg.take();

        return match msg {
            RouteTransportMessage::Route(mut msg) => {
                let tcp_addr = msg.onward_route.addrs.remove(0);
                let key = serde_bare::to_vec::<RouterAddress>(&tcp_addr).unwrap();
                let addr = self.registry.get(&key);

                if addr.is_none() {
                    return Err(RouterError::NoSuchKey.into());
                }
                let addr = addr.unwrap().clone();
                if ctx
                    .send_message(addr.clone(), TcpWorkerMessage::SendMessage(msg))
                    .await
                    .is_err()
                {
                    return Err(TransportError::ConnectionClosed.into());
                }
                if ctx
                    .send_message(addr, TcpWorkerMessage::Receive)
                    .await
                    .is_err()
                {
                    return Err(TransportError::ConnectionClosed.into());
                }
                Ok(())
            }
            _ => Ok(()),
        };
    }
}
