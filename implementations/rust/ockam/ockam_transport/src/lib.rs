use crate::transport_traits::Connection;
use async_trait::async_trait;
use ockam::{Address, Context, Result, Worker};
use ockam_router::message::{Route, RouterAddress, RouterMessage};
use serde::{Deserialize, Serialize};

pub mod transport_traits;

pub struct Transport {
    pub connection: Box<dyn Connection>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum TransportMessage {
    Send(Vec<RouterAddress>, Vec<u8>), // to/from peer worker addresses
}

#[async_trait]
impl Worker for Transport {
    type Message = TransportMessage;
    type Context = Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        match msg {
            TransportMessage::Send(addrs, payload) => {
                let mut rm = RouterMessage::new();
                rm.onward_route = Route { addrs };
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Builder;

    async fn run_connect_test(addr: String) {}

    #[test]
    fn connect() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            run_connect_test(String::from("127.0.0.1:4052")).await;
        });
    }
}
