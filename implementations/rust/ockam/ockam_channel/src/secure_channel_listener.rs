use crate::{SecureChannelNewKeyExchanger, SecureChannelVault, SecureChannelWorker};
use async_trait::async_trait;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{Address, LocalMessage, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[cfg(not(feature = "std"))]
use ockam_core::compat::rand::random;
#[cfg(feature = "std")]
use rand::random;

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
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CreateResponderChannelMessage {
    payload: Vec<u8>,
    completed_callback_address: Option<Address>,
}

impl CreateResponderChannelMessage {
    /// Channel information.
    pub fn payload(&self) -> &Vec<u8> {
        &self.payload
    }
    /// Callback Address
    pub fn completed_callback_address(&self) -> &Option<Address> {
        &self.completed_callback_address
    }
}

impl CreateResponderChannelMessage {
    /// Create message using payload and callback_address
    pub fn new(payload: Vec<u8>, completed_callback_address: Option<Address>) -> Self {
        CreateResponderChannelMessage {
            payload,
            completed_callback_address,
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
        let reply = msg.return_route().clone();
        let msg = msg.body();

        let address_remote: Address = random();
        let address_local: Address = random();

        debug!(
            "Starting SecureChannel responder at local: {}, remote: {}",
            &address_local, &address_remote
        );

        let channel = SecureChannelWorker::new(
            false,
            reply.clone(),
            address_remote.clone(),
            address_local.clone(),
            msg.completed_callback_address().clone(),
            None,
            self.new_key_exchanger.responder()?,
            self.vault.clone(),
        )?;

        ctx.start_worker(vec![address_remote.clone(), address_local], channel)
            .await?;

        // We want this message's return route lead to the remote channel worker, not listener
        let msg = TransportMessage::v1(address_remote, reply, msg.payload().encode()?);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(())
    }
}
