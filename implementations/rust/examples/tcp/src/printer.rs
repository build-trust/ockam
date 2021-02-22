use async_trait::async_trait;
use ockam::Context;
use ockam::Worker;
use ockam_router::message::{RouteableAddress, RouterMessage};
use ockam_transport_tcp::Connection;
use serde::{Deserialize, Serialize};

pub struct Printer {
    pub connection: Box<dyn Connection>,
    pub count: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum PrinterMessage {
    Send(Vec<u8>),
    Receive,
}

#[async_trait]
impl Worker for Printer {
    type Message = PrinterMessage;
    type Context = Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> ockam::Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> ockam::Result<()> {
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Self::Message,
    ) -> ockam::Result<()> {
        return match (msg) {
            PrinterMessage::Send(text) => {
                let mut reply = RouterMessage::new();
                reply.onward_address(RouteableAddress::Local(b"printer".to_vec()));
                reply.return_address(RouteableAddress::Local(b"printer".to_vec()));
                reply.payload = text;
                self.connection.send_message(reply).await?;
                println!("sent \"hello\"");
                self.count += 1;
                if self.count == 2 {
                    ctx.stop().await.unwrap();
                }
                Ok(())
            }
            PrinterMessage::Receive => {
                let m = self.connection.receive_message().await?;
                println!("received \"{}\"", String::from_utf8(m.payload).unwrap());
                self.count += 1;
                if self.count == 2 {
                    ctx.stop().await.unwrap();
                }
                Ok(())
            }
        };
    }
}
