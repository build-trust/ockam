use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

pub const CHANNEL_FACTORY_ADDRESS: &str = "channel_factory";

pub struct ChannelFactory {}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelFactoryMessage {
    Initiate(Address),
    Respond(Address),
}

#[async_trait]
impl Worker for ChannelFactory {
    type Message = ChannelFactoryMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, _ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            ChannelFactoryMessage::Initiate(_local_connection_worker) => {
                // Create and start a new channel worker
                // The worker will need the connection address and to know whether to initiate or respond
                // How to pass in the vault and key exchanger are open questions since they don't
                // (and maybe can't?) implement Clone, Serialize, and Deserialize

                Ok(())
            }
            ChannelFactoryMessage::Respond(_a) => {
                // Create and start a new channel worker
                // The worker will need the connection address and to know whether to initiate or respond

                Ok(())
            }
        };
    }
}
