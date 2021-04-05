use crate::{SecureChannel, SecureChannelMessage};
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

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
        /// Channel ID.
        channel_id: String,
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
                channel_id,
                payload,
            } => {
                let address: Address = channel_id.clone().into();

                let channel = SecureChannel::new(false, reply.clone(), channel_id, None);

                ctx.start_worker(address.clone(), channel).await?;

                // We want this message's return route lead to the remote channel worker, not listener
                let payload = SecureChannelMessage::KeyExchange { payload }.encode()?;
                let msg = TransportMessage {
                    version: 1,
                    onward: address.into(),
                    return_: reply,
                    payload,
                };

                ctx.forward_message(msg).await?;

                Ok(())
            }
        }
    }
}
