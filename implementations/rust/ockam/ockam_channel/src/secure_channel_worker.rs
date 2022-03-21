use crate::{
    CreateResponderChannelMessage, SecureChannelError, SecureChannelKeyExchanger,
    SecureChannelLocalInfo,
};
use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{
    Address, Any, Decodable, Encodable, LocalMessage, Message, Result, Route, Routed,
    TransportMessage, Worker,
};
use ockam_key_exchange_core::{Cipher, CompletedKeyExchange};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// SecureChannel is an abstraction responsible for sending messages (usually over the network) in
/// encrypted and authenticated way.
/// SecureChannel always has two ends: initiator and responder.
pub struct SecureChannelWorker<K: SecureChannelKeyExchanger> {
    is_initiator: bool,
    remote_route: Route,
    address_remote: Address,
    address_local: Address,
    // Optional address to which message is sent after SecureChannel is created
    key_exchange_completed_callback_route: Option<Address>,
    // Optional address to which responder can talk to after SecureChannel is created
    first_responder_address: Option<Address>,
    key_exchanger: Option<K>,
    key_exchange_name: String,
    completed_key_exchange: Option<CompletedKeyExchange<K::Cipher>>,
}

impl<K: SecureChannelKeyExchanger> SecureChannelWorker<K> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        is_initiator: bool,
        remote_route: Route,
        address_remote: Address,
        address_local: Address,
        key_exchange_completed_callback_route: Option<Address>,
        first_responder_address: Option<Address>,
        key_exchanger: K,
    ) -> Result<Self> {
        let key_exchange_name = key_exchanger.name().await?;
        Ok(SecureChannelWorker {
            is_initiator,
            remote_route,
            address_remote,
            address_local,
            key_exchange_completed_callback_route,
            first_responder_address,
            key_exchanger: Some(key_exchanger),
            key_exchange_name,
            completed_key_exchange: None,
        })
    }

    async fn send_key_exchange_payload(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        payload: Vec<u8>,
        is_first_initiator_msg: bool,
    ) -> Result<()> {
        if is_first_initiator_msg {
            // First message from initiator goes to the channel listener
            ctx.send_from_address(
                self.remote_route.clone(),
                CreateResponderChannelMessage::new(payload, self.first_responder_address.take()),
                self.address_remote.clone(),
            )
            .await
        } else {
            // Other messages go to the channel worker itself
            ctx.send_from_address(
                self.remote_route.clone(),
                payload,
                self.address_remote.clone(),
            )
            .await
        }
    }

    async fn handle_encrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!("SecureChannel received Encrypt");

        let reply = msg.return_route();
        let mut onward_route = msg.onward_route();
        let transport_message = msg.into_transport_message();
        let payload = transport_message.payload;

        let _ = onward_route.step();

        let msg = TransportMessage::v1(onward_route, reply, payload.to_vec());
        let payload = msg.encode()?;

        let cipher = self
            .completed_key_exchange
            .as_mut()
            .ok_or(SecureChannelError::KeyExchangeNotComplete)?
            .encryption_cipher();

        let payload = cipher.encrypt_with_ad(&[], payload.as_slice()).await?;

        ctx.send_from_address(
            self.remote_route.clone(),
            payload,
            self.address_remote.clone(),
        )
        .await
    }

    async fn handle_decrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!("SecureChannel received Decrypt");

        let transport_message = msg.into_transport_message();
        let payload = transport_message.payload;
        let payload = Vec::<u8>::decode(&payload)?;

        let cipher = self
            .completed_key_exchange
            .as_mut()
            .ok_or(SecureChannelError::KeyExchangeNotComplete)?
            .decryption_cipher();

        let payload = cipher.decrypt_with_ad(&[], &payload).await?;

        let mut transport_message = TransportMessage::decode(&payload)?;

        transport_message
            .return_route
            .modify()
            .prepend(self.address_local.clone());

        let local_info = SecureChannelLocalInfo::new(self.key_exchange_name.clone());

        let local_msg = LocalMessage::new(transport_message, vec![local_info.to_local_info()?]);

        ctx.forward(local_msg).await
    }

    async fn handle_key_exchange(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        // Received key exchange message from remote channel, need to forward it to local key exchange
        debug!("SecureChannel received KeyExchangeRemote");

        let reply = msg.return_route();
        let transport_message = msg.into_transport_message();
        let payload = transport_message.payload;
        let payload = Vec::<u8>::decode(&payload)?;

        // Update route to a remote
        self.remote_route = reply;

        let key_exchanger;
        if let Some(k) = self.key_exchanger.as_mut() {
            key_exchanger = k;
        } else {
            return Err(SecureChannelError::InvalidInternalState.into());
        }
        let _ = key_exchanger.handle_response(payload.as_slice()).await?;

        if !key_exchanger.is_complete().await? {
            let payload = key_exchanger.generate_request(&[]).await?;
            self.send_key_exchange_payload(ctx, payload, false).await?;
        }

        let key_exchanger;
        if let Some(k) = self.key_exchanger.as_mut() {
            key_exchanger = k;
        } else {
            return Err(SecureChannelError::InvalidInternalState.into());
        }
        if key_exchanger.is_complete().await? {
            let key_exchanger;
            if let Some(k) = self.key_exchanger.take() {
                key_exchanger = k;
            } else {
                return Err(SecureChannelError::InvalidInternalState.into());
            }
            let completed_key_exchange = key_exchanger.finalize().await?;
            let auth_hash = *completed_key_exchange.h();
            self.completed_key_exchange = Some(completed_key_exchange);

            let role_str = if self.is_initiator {
                "initiator"
            } else {
                "responder"
            };

            info!(
                "Started SecureChannel {} at local: {}, remote: {}",
                role_str, &self.address_local, &self.address_remote
            );

            // Notify interested worker about finished key exchange
            if let Some(r) = self.key_exchange_completed_callback_route.take() {
                ctx.send_from_address(
                    r,
                    KeyExchangeCompleted {
                        address: self.address_local.clone(),
                        auth_hash,
                    },
                    self.address_local.clone(),
                )
                .await?;
            }
        }

        Ok(())
    }
}

/// Key Exchange completed message
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Message)]
pub struct KeyExchangeCompleted {
    address: Address,
    auth_hash: [u8; 32],
}

impl KeyExchangeCompleted {
    /// Secure Channel address
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Authentication hash
    pub fn auth_hash(&self) -> [u8; 32] {
        self.auth_hash
    }
}

#[async_trait]
impl<K: SecureChannelKeyExchanger> Worker for SecureChannelWorker<K> {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if self.is_initiator {
            if let Some(initiator) = self.key_exchanger.as_mut() {
                let payload = initiator.generate_request(&[]).await?;

                self.send_key_exchange_payload(ctx, payload, true).await?;
            } else {
                return Err(SecureChannelError::InvalidInternalState.into());
            }
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.address_local {
            self.handle_encrypt(ctx, msg).await?;
        } else if msg_addr == self.address_remote {
            if self.completed_key_exchange.is_none() {
                self.handle_key_exchange(ctx, msg).await?;
            } else {
                self.handle_decrypt(ctx, msg).await?;
            }
        }
        Ok(())
    }
}
