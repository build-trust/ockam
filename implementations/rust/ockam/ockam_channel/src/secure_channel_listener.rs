use crate::SecureChannel;
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use ockam_vault_sync_core::VaultSync;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// SecureChannelListener listens for messages from SecureChannel initiators
/// and creates responder SecureChannels
pub struct SecureChannelListener {
    vault: VaultSync,
}

impl SecureChannelListener {
    /// Create a new SecureChannelListener.
    pub fn new(vault: VaultSync) -> Self {
        Self { vault }
    }
}

/// SecureChannelListener message wrapper.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CreateResponderChannelMessage {
    payload: Vec<u8>,
    callback_address: Option<Address>,
}

impl CreateResponderChannelMessage {
    /// Channel information.
    pub fn payload(&self) -> &Vec<u8> {
        &self.payload
    }
    /// Callback Address
    pub fn callback_address(&self) -> &Option<Address> {
        &self.callback_address
    }
}

impl CreateResponderChannelMessage {
    /// Create message using payload and callback_address
    pub fn new(payload: Vec<u8>, callback_address: Option<Address>) -> Self {
        CreateResponderChannelMessage {
            payload,
            callback_address,
        }
    }
}

#[async_trait]
impl Worker for SecureChannelListener {
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

        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.start_another()?);

        let channel = SecureChannel::new(
            false,
            reply.clone(),
            address_remote.clone(),
            address_local.clone(),
            msg.callback_address()
                .as_ref()
                .map(|a| Route::new().append(a.clone()).into()),
            &new_key_exchanger,
            self.vault.start_another()?,
        )?;

        ctx.start_worker(vec![address_remote.clone(), address_local], channel)
            .await?;

        // We want this message's return route lead to the remote channel worker, not listener
        let msg = TransportMessage {
            version: 1,
            onward_route: address_remote.into(),
            return_route: reply,
            payload: msg.payload().encode()?,
        };

        ctx.forward(msg).await?;

        Ok(())
    }
}
