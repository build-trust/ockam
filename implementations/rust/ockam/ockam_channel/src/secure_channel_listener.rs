use crate::{SecureChannel, SecureChannelMessage};
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use rand::random;

/// SecureChannelListener listens for messages from SecureChannel initiators
/// and creates responder SecureChannels
pub struct SecureChannelListener;

impl SecureChannelListener {
    /// Create a new SecureChannelListener.
    pub fn new() -> Self {
        Self {}
    }
}

/// SecureChannelListener message wrapper.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum SecureChannelListenerMessage {
    /// Create a new responder channel.
    CreateResponderChannel {
        /// Channel information.
        payload: Vec<u8>,
    },
}

#[async_trait]
impl Worker for SecureChannelListener {
    type Message = SecureChannelListenerMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let reply = msg.reply().clone();
        match msg.take() {
            SecureChannelListenerMessage::CreateResponderChannel {
                payload,
            } => {
                let address: Address = random();
                let address_str: String = address.clone().into(); // FIXME

                let channel = SecureChannel::new(false, reply.clone(), address_str, None);

                ctx.start_worker(address.clone(), channel).await?;

                // We want this message's return route lead to the remote channel worker, not listener
                let payload = SecureChannelMessage::KeyExchange { payload }.encode()?;
                let msg = TransportMessage {
                    version: 1,
                    onward_route: address.into(),
                    return_route: reply,
                    payload,
                };

                ctx.forward_message(msg).await?;

                Ok(())
            }
        }
    }
}
