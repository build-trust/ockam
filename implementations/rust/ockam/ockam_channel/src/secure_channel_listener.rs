use crate::SecureChannel;
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Routed, TransportMessage, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use ockam_vault::SoftwareVault;
use ockam_vault_core::ErrorVault;
use ockam_vault_sync_core::Vault;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::info;

/// SecureChannelListener listens for messages from SecureChannel initiators
/// and creates responder SecureChannels
pub struct SecureChannelListener {
    vault_worker_address: Address,
}

impl SecureChannelListener {
    /// Create a new SecureChannelListener.
    pub fn new(vault_worker_address: Address) -> Self {
        Self {
            vault_worker_address,
        }
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

                info!(
                    "Starting SecureChannel responder at local: {}, remote: {}",
                    &address_local, &address_remote
                );

                let vault = Vault::create(
                    ctx,
                    self.vault_worker_address.clone(),
                    SoftwareVault::error_domain(), /* FIXME */
                )
                .await?;

                let new_key_exchanger = XXNewKeyExchanger::new(vault.start_another()?);

                let channel = SecureChannel::new(
                    false,
                    reply.clone(),
                    address_remote.clone(),
                    address_local.clone(),
                    None,
                    &new_key_exchanger,
                    vault,
                )?;

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
