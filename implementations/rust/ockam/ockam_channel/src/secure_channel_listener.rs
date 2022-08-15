use crate::{SecureChannelDecryptor, SecureChannelNewKeyExchanger, SecureChannelVault};
use ockam_core::async_trait;
use ockam_core::compat::rand::random;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{
    Address, Encodable, LocalMessage, Message, Result, Routed, TransportMessage, Worker,
};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// SecureChannelListener listens for messages from SecureChannel initiators
/// and creates responder SecureChannels
pub struct SecureChannelListener<V: SecureChannelVault, N: SecureChannelNewKeyExchanger> {
    new_key_exchanger: N,
    vault: V,
}

impl<V: SecureChannelVault, N: SecureChannelNewKeyExchanger> SecureChannelListener<V, N> {
    /// Create a new SecureChannelListener.
    pub fn new(new_key_exchanger: N, vault: V) -> Self {
        Self {
            new_key_exchanger,
            vault,
        }
    }
}

/// SecureChannelListener message wrapper.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Message)]
pub struct CreateResponderChannelMessage {
    payload: Vec<u8>,
    custom_payload: Option<Vec<u8>>,
}

impl CreateResponderChannelMessage {
    /// Channel information.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    /// Callback Address
    pub fn custom_payload(&self) -> &Option<Vec<u8>> {
        &self.custom_payload
    }
}

impl CreateResponderChannelMessage {
    /// Create message using payload and callback_address
    pub fn new(payload: Vec<u8>, custom_payload: Option<Vec<u8>>) -> Self {
        CreateResponderChannelMessage {
            payload,
            custom_payload,
        }
    }
}

#[async_trait]
impl<V: SecureChannelVault, N: SecureChannelNewKeyExchanger> Worker
    for SecureChannelListener<V, N>
{
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route().clone();
        let msg = msg.body();

        let address_remote: Address = random();

        debug!(
            "Starting SecureChannel responder at remote: {}",
            &address_remote
        );

        let key_exchanger = self.new_key_exchanger.responder().await?;
        let vault = self.vault.async_try_clone().await?;
        let decryptor = SecureChannelDecryptor::new_responder(key_exchanger, None, vault).await?;

        ctx.start_worker(vec![address_remote.clone()], decryptor)
            .await?;

        // We want this message's return route lead to the remote channel worker, not listener
        let msg = TransportMessage::v1(address_remote, return_route, msg.payload().encode()?);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(())
    }
}
