use crate::SecureChannel;
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use rand::random;
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
        let reply = msg.return_route().clone();
        match msg.body() {
            SecureChannelListenerMessage::CreateResponderChannel { payload } => {
                let address_remote: Address = random();
                let address_local: Address = random();

                let channel = SecureChannel::new(
                    false,
                    reply.clone(),
                    address_remote.clone(),
                    address_local.clone(),
                    None,
                );

                ctx.start_worker(vec![address_remote.clone(), address_local], channel)
                    .await?;

                // We want this message's return route lead to the remote channel worker, not listener
                let msg = TransportMessage {
                    version: 1,
                    onward_route: address_remote.into(),
                    return_route: reply,
                    payload: payload.encode()?,
                };

                ctx.forward(msg).await?;

                Ok(())
            }
        }
    }
}
