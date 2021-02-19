#![allow(unused)]
use async_trait::async_trait;
use ockam::{Address, Context, Result, Worker};
use ockam_router::message::{Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_LOCAL};
use ockam_transport_tcp::Connection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PlayerMessage {
    Serve(Route),
    Return,
    Hit,
}

pub struct Player {
    pub connection: Box<dyn Connection>,
    pub return_route: Route,
    pub counter: usize,
}

impl Player {
    pub async fn process_message(
        &mut self,
        ctx: &mut <Player as Worker>::Context,
        msg: PlayerMessage,
    ) {
        println!("Player got msg: {:?}", msg);
        match msg {
            PlayerMessage::Serve(r) => {
                let m = RouterMessage {
                    version: 1,
                    onward_route: r,
                    return_route: Route {
                        addrs: vec![RouterAddress {
                            address_type: ROUTER_ADDRESS_LOCAL,
                            address: "server".into(),
                        }],
                    },
                    payload: "serve".into(),
                };
                self.connection.send_message(m).await.unwrap();
            }
            PlayerMessage::Return => {
                let m = self.connection.receive_message().await.unwrap();
                println!("{}", String::from_utf8(m.payload).unwrap());
                self.return_route = m.return_route.clone();
                ctx.send_message("server", PlayerMessage::Hit);
            }
            PlayerMessage::Hit => {
                ctx.send_message("server", PlayerMessage::Return);
                let m = RouterMessage {
                    version: 1,
                    onward_route: self.return_route.clone(),
                    return_route: Route {
                        addrs: vec![RouterAddress {
                            address_type: ROUTER_ADDRESS_LOCAL,
                            address: "server".into(),
                        }],
                    },
                    payload: "bam".into(),
                };
                self.connection.send_message(m).await.unwrap();
            }
        }
    }
}

#[async_trait]
impl Worker for Player {
    type Message = PlayerMessage;
    type Context = Context;

    fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        println!("Starting player {}", ctx.address());
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        println!("got message {:?}", msg);
        match msg {
            PlayerMessage::Serve(r) => {
                let m = RouterMessage {
                    version: 1,
                    onward_route: r,
                    return_route: Route {
                        addrs: vec![RouterAddress {
                            address_type: ROUTER_ADDRESS_LOCAL,
                            address: "server".into(),
                        }],
                    },
                    payload: "serve".into(),
                };
                self.connection.send_message(m).await.unwrap();
            }
            PlayerMessage::Return => {
                let m = self.connection.receive_message().await.unwrap();
                println!("{}", String::from_utf8(m.payload).unwrap());
                self.return_route = m.return_route.clone();
                ctx.send_message("server", PlayerMessage::Hit);
            }
            PlayerMessage::Hit => {
                ctx.send_message("server", PlayerMessage::Return);
                let m = RouterMessage {
                    version: 1,
                    onward_route: self.return_route.clone(),
                    return_route: Route {
                        addrs: vec![RouterAddress {
                            address_type: ROUTER_ADDRESS_LOCAL,
                            address: "server".into(),
                        }],
                    },
                    payload: "bam".into(),
                };
                self.connection.send_message(m).await.unwrap();
            }
        }
        Ok(())
    }
}
