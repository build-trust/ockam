use crate::{RouterAddress, RouterError, TransportMessage};
use async_trait::async_trait;
use hashbrown::HashMap;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

pub const ROUTER_ADDRESS: &str = "router";
pub const LOCAL_ROUTER_ADDRESS: &str = "local_router";

pub const MAX_ROUTER_TYPES: usize = 256;

pub const ROUTER_ADDRESS_TYPE_LOCAL: u8 = 0;
pub const ROUTER_ADDRESS_TYPE_TCP: u8 = 1;

pub type RouterType = usize;

/// This is the message type for all workers that handle
/// TransportMessages.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum RouteTransportMessage {
    Route(TransportMessage),
    Ping,
}

pub struct Router {
    registry: HashMap<u8, Address>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            registry: HashMap::new(),
        }
    }
    pub fn register(&mut self, addr_type: u8, address: Address) -> Result<()> {
        if let Some(_) = self.registry.get(&addr_type.clone()) {
            return Err(RouterError::TypeIdInUse.into());
        }
        self.registry.insert(addr_type.clone(), address.clone());
        Ok(())
    }
}

pub fn print_route(addrs: &Vec<RouterAddress>) {
    println!("Printing route. Length{}", addrs.len());
    for a in addrs {
        println!(
            "Type: {:?} {} {:?}",
            a.address_type,
            String::from_utf8(a.address.clone()).unwrap_or_else(|_| String::from("no string")),
            a.address
        )
    }
}

#[async_trait]
impl Worker for Router {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        router_msg: Self::Message,
    ) -> Result<()> {
        return match router_msg {
            RouteTransportMessage::Route(m) => {
                if m.onward_route.addrs.is_empty() {
                    return Err(RouterError::NoRoute.into());
                }
                let next_hop = m.onward_route.addrs.get(0).unwrap();
                let handler_addr = self.registry.get(&next_hop.address_type);
                if handler_addr.is_none() {
                    return Err(RouterError::NoSuchKey.into());
                }
                let handler_addr = handler_addr.unwrap();
                let r = ctx
                    .send_message(handler_addr.clone(), RouteTransportMessage::Route(m))
                    .await;
                match r {
                    Ok(..) => Ok(()),
                    Err(e) => Err(e.into()),
                }
            }
            RouteTransportMessage::Ping => Ok(()),
        };
    }
}

// #[cfg(test)]
// mod test {
//     use crate::message::{RouteableAddress, RouterAddress, TransportMessage, ROUTER_ADDRESS_TYPE_LOCAL};
//     use crate::router::{RouteMessage, Router};
//     use async_trait::async_trait;
//     use ockam::{Address, Result, Worker};
//     use crate::ROUTER_ADDRESS_TYPE_LOCAL;
//
//     pub struct MyWorker {
//         pub address: String,
//         pub router: String,
//         pub first: bool,
//         pub text: String,
//         pub count: usize,
//         pub is_first: bool,
//     }
//
//     #[async_trait]
//     impl Worker for MyWorker {
//         type Message = RouteMessage;
//         type Context = ockam::Context;
//
//         async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
//             self.count = 0;
//             return Ok(());
//         }
//
//         fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
//             Ok(())
//         }
//
//         async fn handle_message(
//             &mut self,
//             ctx: &mut Self::Context,
//             mut msg: Self::Message,
//         ) -> Result<()> {
//             let mut msg = msg.router_message;
//             println!("{}", String::from_utf8(msg.payload.clone()).unwrap());
//             if msg.onward_route.addrs.is_empty() {
//                 if self.count > 0 && self.is_first {
//                     ctx.stop().await?;
//                     return Ok(());
//                 } else {
//                     msg.onward_route = msg.return_route.clone();
//                     msg.return_route.addrs.truncate(0);
//                 }
//             }
//             let mut p = msg.payload.clone();
//             p.append(&mut self.text.clone().as_bytes().to_vec());
//             msg.payload = p;
//             self.count += 1;
//             let address: ockam::Address = self.address.clone().into();
//             let r: Address = self.router.clone().into();
//             msg.return_route.addrs.insert(
//                 0,
//                 RouterAddress {
//                     address_type: ROUTER_ADDRESS_TYPE_LOCAL,
//                     address: ctx.address().to_vec(),
//                 },
//             );
//             ctx.send_message(
//                 r,
//                 RouteMessage {
//                     router_message: msg,
//                 },
//             )
//             .await?;
//             Ok(())
//         }
//     }
//
//     #[test]
//     fn route() {
//         let (ctx, mut exe) = ockam::start_node();
//         exe.execute(async move {
//             let router = Router {};
//             let w1 = MyWorker {
//                 address: String::from("w1"),
//                 router: String::from("router"),
//                 first: false,
//                 text: "1".to_string(),
//                 count: 0,
//                 is_first: true,
//             };
//             let w2 = MyWorker {
//                 address: String::from("w2"),
//                 router: String::from("router"),
//                 first: false,
//                 text: "2".to_string(),
//                 count: 0,
//                 is_first: false,
//             };
//             let w3 = MyWorker {
//                 address: String::from("w3"),
//                 router: String::from("router"),
//                 first: false,
//                 text: "3".to_string(),
//                 count: 0,
//                 is_first: false,
//             };
//             ctx.start_worker("router", router).await.unwrap();
//             ctx.start_worker(w1.address.clone(), w1).await.unwrap();
//             ctx.start_worker(w2.address.clone(), w2).await.unwrap();
//             ctx.start_worker(w3.address.clone(), w3).await.unwrap();
//
//             let mut m = TransportMessage::new();
//
//             m.onward_address(RouteableAddress::Local(b"w1".to_vec()));
//             m.onward_address(RouteableAddress::Local(b"w2".to_vec()));
//             m.onward_address(RouteableAddress::Local(b"w3".to_vec()));
//             m.payload = b"0".to_vec();
//
//             ctx.send_message(String::from("router"), RouteMessage { router_message: m })
//                 .await
//                 .unwrap();
//         })
//         .unwrap();
//     }
// }
