use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use serde::{Deserialize, Serialize};

pub struct Channel {
    _addr_encrypted: Address, // messages coming in on this address need to be decrypted
    _addr_clear: Address,     // messages coming in on this address need to be encrypted
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelMessage {
    Initiate(Address),
    Respond(Address),
}

#[async_trait]
impl Worker for Channel {
    type Message = ChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, _ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            ChannelMessage::Initiate(_connection_address) => {
                // You should be able to create an initiator here and wait for it to complete
                // the exchange (see the key exchange example).
                // Problems to be solved: how to pass in the vault and key exchange (you can
                // hard-code it for now) and how to get the CompletedKeyExchange back from
                // the initiator.

                Ok(())
            }
            ChannelMessage::Respond(_connection_address) => {
                // Same comments as above
                Ok(())
            }
        };
    }
}
