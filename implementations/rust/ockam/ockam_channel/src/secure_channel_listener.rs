use crate::{SecureChannelDecryptor, SecureChannelNewKeyExchanger, SecureChannelVault};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, AllowAll, DenyAll, LocalDestinationOnly, Mailbox, Mailboxes};
use ockam_core::{Address, Message, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};
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

        let address_remote = Address::random_tagged("SecureChannel.responder.decryptor.remote");
        let address_internal = Address::random_tagged("SecureChannel.responder.decryptor.internal");

        debug!(
            "Starting SecureChannel responder at remote: {}",
            &address_remote
        );

        let key_exchanger = self.new_key_exchanger.responder().await?;
        let vault = self.vault.async_try_clone().await?;

        let decryptor = SecureChannelDecryptor::new_responder(
            key_exchanger,
            address_remote.clone(),
            address_internal.clone(),
            None,
            msg.payload,
            return_route,
            vault,
            vec![],
        )
        .await?;

        let remote_mailbox = Mailbox::new(
            address_remote.clone(),
            // Doesn't matter since we check incoming messages cryptographically,
            // but this may be reduced to allowing only from the transport connection that was used
            // to create this channel initially
            Arc::new(AllowAll),
            // Communicate to the other side of the channel
            Arc::new(AllowAll),
        );
        let internal_mailbox = Mailbox::new(
            address_internal,
            Arc::new(DenyAll),
            // Prevent exploit of using our node as an authorized proxy
            Arc::new(LocalDestinationOnly),
        );
        WorkerBuilder::with_mailboxes(
            Mailboxes::new(remote_mailbox, vec![internal_mailbox]),
            decryptor,
        )
        .start(ctx)
        .await?;

        Ok(())
    }
}
