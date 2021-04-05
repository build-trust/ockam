use crate::{SecureChannel, SecureChannelMessage};
use async_trait::async_trait;
use ockam::{Context, TransportMessage, Worker};
use ockam_core::{Address, Message, Result, Routed};
use serde::{Deserialize, Serialize};

/// SecureChannel listener
pub struct SecureChannelListener;

impl SecureChannelListener {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum SecureChannelListenerMessage {
    CreateResponderChannel {
        channel_id: String,
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
