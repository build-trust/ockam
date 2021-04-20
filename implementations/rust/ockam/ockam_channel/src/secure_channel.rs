use crate::{SecureChannelError, SecureChannelListener, SecureChannelListenerMessage};
use async_trait::async_trait;
use ockam_core::{Address, Any, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_key_exchange_core::{KeyExchanger, NewKeyExchanger};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use ockam_vault::SoftwareVault;
use ockam_vault_core::{Secret, SymmetricVault};
use ockam_vault_sync_core::VaultSync;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

struct ChannelKeys {
    encrypt_key: Secret,
    decrypt_key: Secret,
    nonce: u16,
}

/// SecureChannel info returned from start_initiator_channel
/// Auth hash can be used for further authentication of the channel
/// and tying it up cryptographically to some source of Trust. (e.g. Entities)
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct SecureChannelInfo {
    worker_address: Address,
    auth_hash: [u8; 32],
}

impl SecureChannelInfo {
    /// Return a clone of the worker's address.
    pub fn address(&self) -> Address {
        self.worker_address.clone()
    }
    /// Return the auth hash.
    pub fn auth_hash(&self) -> [u8; 32] {
        self.auth_hash
    }
}

/// SecureChannel is an abstraction responsible for sending messages (usually over the network) in
/// encrypted and authenticated way.
/// SecureChannel always has two ends: initiator and responder.
pub struct SecureChannel {
    is_initiator: bool,
    remote_route: Route,
    address_remote: Address,
    address_local: Address,
    keys: Option<ChannelKeys>,
    key_exchange_completed_callback_route: Option<Route>,
    vault: VaultSync,
    key_exchanger: Option<Box<dyn KeyExchanger + Send + 'static>>,
}

impl SecureChannel {
    pub(crate) fn new<N: NewKeyExchanger>(
        is_initiator: bool,
        remote_route: Route,
        address_remote: Address,
        address_local: Address,
        key_exchange_completed_callback_route: Option<Route>,
        new_key_exchanger: &N,
        vault: VaultSync,
    ) -> Result<Self> {
        let key_exchanger: Box<dyn KeyExchanger + Send + 'static> = if is_initiator {
            Box::new(new_key_exchanger.initiator()?)
        } else {
            Box::new(new_key_exchanger.responder()?)
        };

        Ok(SecureChannel {
            is_initiator,
            remote_route,
            address_remote,
            address_local,
            keys: None,
            key_exchange_completed_callback_route,
            key_exchanger: Some(key_exchanger),
            vault,
        })
    }

    /// Create and start channel listener with given address.
    pub async fn create_listener<A: Into<Address>>(
        ctx: &Context,
        address: A,
        vault_worker_address: Address,
    ) -> Result<()> {
        let channel_listener = SecureChannelListener::new(vault_worker_address);
        let address = address.into();
        info!("Starting SecureChannel listener at {}", &address);
        ctx.start_worker(address, channel_listener).await
    }

    /// Create initiator channel with given route to a remote channel listener.
    pub async fn create<A: Into<Route>>(
        ctx: &mut Context,
        route: A,
        vault_worker_address: Address,
    ) -> Result<SecureChannelInfo> {
        let address_remote: Address = random();
        let address_local: Address = random();

        info!(
            "Starting SecureChannel initiator at local: {}, remote: {}",
            &address_local, &address_remote
        );

        let vault = VaultSync::create(
            ctx,
            vault_worker_address,
            SoftwareVault::error_domain_static(), /* FIXME */
        )
        .await?;

        let new_key_exchanger = XXNewKeyExchanger::new(vault.start_another()?);

        let channel = SecureChannel::new(
            true,
            route.into(),
            address_remote.clone(),
            address_local.clone(),
            Some(Route::new().append(ctx.address()).into()),
            &new_key_exchanger,
            vault,
        )?;

        ctx.start_worker(vec![address_remote, address_local.clone()], channel)
            .await?;

        let resp = ctx
            .receive_match(|m: &KeyExchangeCompleted| m.address == address_local)
            .await?
            .take()
            .body();

        let info = SecureChannelInfo {
            worker_address: address_local,
            auth_hash: resp.auth_hash,
        };

        Ok(info)
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
                SecureChannelListenerMessage::CreateResponderChannel { payload },
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct KeyExchangeCompleted {
    address: Address,
    auth_hash: [u8; 32],
}

#[async_trait]
impl Worker for SecureChannel {
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
                    let keys = key_exchanger.finalize_box()?;

                    info!("Key exchange completed at {}", &self.address_local);

                    self.keys = Some(ChannelKeys {
                        encrypt_key: keys.encrypt_key().clone(),
                        decrypt_key: keys.decrypt_key().clone(),
                        nonce: 0,
                    });

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
