use crate::{CreateResponderChannelMessage, SecureChannelError, SecureChannelVault};
use async_trait::async_trait;
use ockam_core::{Address, Any, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_key_exchange_core::KeyExchanger;
use ockam_node::Context;
use ockam_vault_core::Secret;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

pub(crate) struct ChannelKeys {
    encrypt_key: Secret,
    decrypt_key: Secret,
    nonce: u16,
}

/// SecureChannel is an abstraction responsible for sending messages (usually over the network) in
/// encrypted and authenticated way.
/// SecureChannel always has two ends: initiator and responder.
pub struct SecureChannelWorker<V: SecureChannelVault, K: KeyExchanger + Send + 'static> {
    is_initiator: bool,
    remote_route: Route,
    address_remote: Address,
    address_local: Address,
    keys: Option<ChannelKeys>,
    // Optional address to which message is sent after SecureChannel is created
    key_exchange_completed_callback_route: Option<Address>,
    // Optional address to which responder can talk to after SecureChannel is created
    first_responder_address: Option<Address>,
    vault: V,
    key_exchanger: Option<K>,
}

impl<V: SecureChannelVault, K: KeyExchanger + Send + 'static> SecureChannelWorker<V, K> {
    pub(crate) fn new(
        is_initiator: bool,
        remote_route: Route,
        address_remote: Address,
        address_local: Address,
        key_exchange_completed_callback_route: Option<Address>,
        first_responder_address: Option<Address>,
        key_exchanger: K,
        vault: V,
    ) -> Result<Self> {
        Ok(SecureChannelWorker {
            is_initiator,
            remote_route,
            address_remote,
            address_local,
            keys: None,
            key_exchange_completed_callback_route,
            first_responder_address,
            key_exchanger: Some(key_exchanger),
            vault,
        })
    }

    fn convert_nonce_u16(nonce: u16) -> ([u8; 2], [u8; 12]) {
        let mut n: [u8; 12] = [0; 12];
        let b: [u8; 2] = nonce.to_be_bytes();
        n[10] = b[0];
        n[11] = b[1];

        (b, n)
    }

    fn convert_nonce_small(b: &[u8]) -> Result<[u8; 12]> {
        if b.len() != 2 {
            return Err(SecureChannelError::InvalidNonce.into());
        }
        let mut n: [u8; 12] = [0; 12];
        n[10] = b[0];
        n[11] = b[1];

        Ok(n)
    }

    fn get_keys(keys: &mut Option<ChannelKeys>) -> Result<&mut ChannelKeys> {
        if let Some(k) = keys.as_mut() {
            Ok(k)
        } else {
            return Err(SecureChannelError::KeyExchangeNotComplete.into());
        }
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
}

/// Key Exchange completed message
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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
impl<V: SecureChannelVault, K: KeyExchanger + Send + 'static> Worker for SecureChannelWorker<V, K> {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if self.is_initiator {
            if let Some(initiator) = self.key_exchanger.as_mut() {
                let payload = initiator.process(&[])?;

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
        let reply = msg.return_route().clone();
        let mut onward_route = msg.onward_route();
        let msg_addr = msg.msg_addr();
        let transport_message = msg.into_transport_message();
        let payload = transport_message.payload;

        if msg_addr == self.address_local {
            debug!("SecureChannel received Encrypt");

            let _ = onward_route.step();

            let msg = TransportMessage {
                version: 1,
                onward_route,
                return_route: reply,
                payload,
            };
            let payload = msg.encode()?;

            let payload = {
                let keys = Self::get_keys(&mut self.keys)?;

                let nonce = keys.nonce;

                if nonce == u16::max_value() {
                    return Err(SecureChannelError::InvalidNonce.into());
                }

                keys.nonce += 1;

                let (small_nonce, nonce) = Self::convert_nonce_u16(nonce);

                let mut cipher_text = self.vault.aead_aes_gcm_encrypt(
                    &keys.encrypt_key,
                    payload.as_slice(),
                    &nonce,
                    &[],
                )?;

                let mut res = Vec::new();
                res.extend_from_slice(&small_nonce);
                res.append(&mut cipher_text);

                res
            };

            ctx.send_from_address(
                self.remote_route.clone(),
                payload,
                self.address_remote.clone(),
            )
            .await?;
        } else if msg_addr == self.address_remote {
            let payload = <Vec<u8> as Message>::decode(&payload)?;
            if self.keys.is_none() {
                // Received key exchange message from remote channel, need to forward it to local key exchange
                debug!("SecureChannel received KeyExchangeRemote");
                // Update route to a remote
                self.remote_route = reply;

                let key_exchanger;
                if let Some(k) = self.key_exchanger.as_mut() {
                    key_exchanger = k;
                } else {
                    return Err(SecureChannelError::InvalidInternalState.into());
                }
                let _ = key_exchanger.process(payload.as_slice())?;

                if !key_exchanger.is_complete() {
                    let payload = key_exchanger.process(&[])?;
                    self.send_key_exchange_payload(ctx, payload, false).await?;
                }

                let key_exchanger;
                if let Some(k) = self.key_exchanger.as_mut() {
                    key_exchanger = k;
                } else {
                    return Err(SecureChannelError::InvalidInternalState.into());
                }
                if key_exchanger.is_complete() {
                    let key_exchanger;
                    if let Some(k) = self.key_exchanger.take() {
                        key_exchanger = k;
                    } else {
                        return Err(SecureChannelError::InvalidInternalState.into());
                    }
                    let keys = key_exchanger.finalize()?;

                    self.keys = Some(ChannelKeys {
                        encrypt_key: keys.encrypt_key().clone(),
                        decrypt_key: keys.decrypt_key().clone(),
                        nonce: 0,
                    });

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
                                auth_hash: keys.h().clone(),
                            },
                            self.address_local.clone(),
                        )
                        .await?;
                    }
                }
            } else {
                debug!("SecureChannel received Decrypt");
                let payload = {
                    let keys = Self::get_keys(&mut self.keys)?;

                    if payload.len() < 2 {
                        return Err(SecureChannelError::InvalidNonce.into());
                    }

                    let nonce = Self::convert_nonce_small(&payload.as_slice()[..2])?;

                    let plain_text = self.vault.aead_aes_gcm_decrypt(
                        &keys.decrypt_key,
                        &payload[2..],
                        &nonce,
                        &[],
                    )?;

                    plain_text
                };

                let mut transport_message = TransportMessage::decode(&payload)?;

                transport_message
                    .return_route
                    .modify()
                    .prepend(self.address_local.clone());

                ctx.forward(transport_message).await?;
            }
        }
        Ok(())
    }
}
