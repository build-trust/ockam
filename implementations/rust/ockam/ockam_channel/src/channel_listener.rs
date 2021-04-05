use crate::channel::{Channel, ChannelMessage};
use async_trait::async_trait;
use ockam::{Context, TransportMessage, Worker};
use ockam_core::{Address, Message, Result, Routed};
use serde::{Deserialize, Serialize};

/// Channel listener for XX key agreement
pub struct XXChannelListener;

impl XXChannelListener {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelListenerMessage {
    CreateResponderChannel {
        channel_id: String,
        payload: Vec<u8>,
    },
}

#[async_trait]
impl Worker for XXChannelListener {
    type Message = ChannelListenerMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let reply = msg.reply().clone();
        match msg.take() {
            ChannelListenerMessage::CreateResponderChannel {
                channel_id,
                payload,
            } => {
                let address: Address = channel_id.clone().into();

                let channel = Channel::new(false, reply.clone(), channel_id, None);

                ctx.start_worker(address.clone(), channel).await?;

                // We want this message's return route lead to the remote channel worker, not listener
                let payload = ChannelMessage::KeyExchange { payload }.encode()?;
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
