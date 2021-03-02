use crate::router::RouteTransportMessage;
use crate::RouterError;
use async_trait::async_trait;
use hashbrown::HashMap;
use ockam::{Address, Context, Worker};
use ockam_core::Result;

pub struct LocalRouter {
    pub registry: HashMap<Vec<u8>, Address>,
}

impl LocalRouter {
    pub fn new() -> Self {
        LocalRouter {
            registry: HashMap::new(),
        }
    }
    pub fn register(&mut self, addr: Address) -> Result<()> {
        let key = addr.to_vec();
        if self.registry.insert(key.clone(), addr).is_some() {
            return Err(RouterError::KeyInUse.into());
        };
        Ok(())
    }
}

#[async_trait]
impl Worker for LocalRouter {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(mut msg) => {
                let local_addr = msg.onward_route.addrs.remove(0);
                if !self.registry.contains_key(&local_addr.address.clone()) {
                    return Err(RouterError::NoSuchType.into());
                }
                let addr = self.registry.get(&local_addr.address.clone()).unwrap();

                if let Err(e) = ctx
                    .send_message(addr.clone(), RouteTransportMessage::Route(msg))
                    .await
                {
                    return Err(e);
                }
                Ok(())
            }
            _ => Ok(()),
        };
    }
}
