use crate::key_exchange::{
    KeyExchangeRequestMessage, KeyExchangeResponseMessage, XInitiator, XResponder,
};
use crate::{SecureChannelError, SecureChannelListener, SecureChannelListenerMessage};
use async_trait::async_trait;
use ockam_core::{Address, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use ockam_vault::SoftwareVault;
use ockam_vault_core::{Secret, SymmetricVault};
use rand::random;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::debug;

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
    pub fn worker_address(&self) -> &Address {
        &self.worker_address
    }
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
    channel_id: String,
    key_exchange_route: Option<Route>, // this address is used to send messages to key exchange worker
    keys: Option<ChannelKeys>,
    key_exchange_completed_callback_route: Option<Route>,
    vault: Arc<Mutex<SoftwareVault>>,
}

impl SecureChannel {
    pub(crate) fn new(
        is_initiator: bool,
        remote_route: Route,
        channel_id: String,
        key_exchange_completed_callback_route: Option<Route>,
    ) -> Self {
        // TODO: Replace with worker
        let vault = Arc::new(Mutex::new(SoftwareVault::new()));
        SecureChannel {
            is_initiator,
            remote_route,
            channel_id,
            key_exchange_route: None,
            keys: None,
            key_exchange_completed_callback_route,
            vault,
        }
    }

    pub async fn create_listener(ctx: &Context, address: Address) -> Result<()> {
        let channel_listener = SecureChannelListener::new();
        ctx.start_worker(address, channel_listener).await
    }

    /// Create initiator channel with given channel id and route to a remote channel listener.
    pub async fn create<A: Into<Route>>(ctx: &mut Context, route: A) -> Result<SecureChannelInfo> {
        let address: Address = random();
        let address_str: String = address.clone().into();

        let channel = SecureChannel::new(
            true,
            route.into(),
            address_str.clone(),
            Some(Route::new().append(ctx.address()).into()),
        );

        ctx.start_worker(address.clone(), channel).await?;

        let resp = ctx
            .receive_match(|m: &KeyExchangeCompleted| m.channel_id == address_str)
            .await?
            .take()
            .take();

        let info = SecureChannelInfo {
            worker_address: address,
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

    async fn handle_key_exchange_local(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        response: KeyExchangeResponseMessage,
        is_first_initiator_msg: bool,
    ) -> Result<()> {
        // Handles response from KeyExchange Worker
        debug!("SecureChannel received KeyExchangeLocal");

        if let Some(payload) = response.payload().clone() {
            debug!("SecureChannel received Payload");
            if is_first_initiator_msg {
                // First message from initiator goes to the channel listener
                ctx.send_message(
                    self.remote_route.clone(),
                    SecureChannelListenerMessage::CreateResponderChannel {
                        channel_id: self.channel_id.clone(),
                        payload,
                    },
                )
                .await?;
            } else {
                // Other messages go to the channel worker itself
                ctx.send_message(
                    self.remote_route.clone(),
                    SecureChannelMessage::KeyExchange { payload },
                )
                .await?;
            };
        }
        if let Some(keys) = response.keys().clone() {
            // We now have shared encryption keys
            debug!("SecureChannel received ExchangeComplete");

            if self.keys.is_some() {
                return Err(SecureChannelError::InvalidInternalState.into());
            }

            let auth_hash = keys.h();
            self.keys = Some(ChannelKeys {
                encrypt_key: Secret::new(keys.encrypt_key()),
                decrypt_key: Secret::new(keys.decrypt_key()),
                nonce: 1,
            });

            // Notify interested worker about finished key exchange
            if let Some(r) = self.key_exchange_completed_callback_route.take() {
                ctx.send_message(
                    r,
                    KeyExchangeCompleted {
                        auth_hash,
                        channel_id: self.channel_id.clone(),
                    },
                )
                .await?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum SecureChannelMessage {
    KeyExchange { payload: Vec<u8> },
    Encrypt { m: Vec<u8> },
    Decrypt { payload: Vec<u8> },
}

impl SecureChannelMessage {
    /// Create message that if sent to the Channel worker, will be encrypted and sent to the remote Channel
    pub fn create_encrypt_message<M: Message>(m: M) -> Result<SecureChannelMessage> {
        let m = SecureChannelMessage::Encrypt { m: m.encode()? };

        Ok(m)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct KeyExchangeCompleted {
    channel_id: String,
    auth_hash: [u8; 32],
}

#[async_trait]
impl Worker for SecureChannel {
    type Message = SecureChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // TODO: Replace key_exchanger and vault with worker
        let key_exchanger = XXNewKeyExchanger::new(self.vault.clone(), self.vault.clone());

        // Spawn Key exchange worker
        let key_exchange_addr: Address = format!("{}/kex", self.channel_id)
            .as_bytes()
            .to_vec()
            .into();

        let key_exchange_route: Route = Route::new().append(key_exchange_addr.clone()).into();
        self.key_exchange_route = Some(key_exchange_route.clone());

        if self.is_initiator {
            let initiator = key_exchanger.initiator();
            let initiator = XInitiator::new(initiator);

            ctx.start_worker(key_exchange_addr, initiator).await?;

            // FIXME: Remove req_id in the future when we fix message without length decode
            let req_id = b"CHANNEL_REQ".to_vec();

            // Kick in initiator to start key exchange process
            ctx.send_message(
                key_exchange_route,
                KeyExchangeRequestMessage::InitiatorFirstMessage {
                    req_id: req_id.clone(),
                },
            )
            .await?;

            let m = ctx
                .receive_match(|m: &KeyExchangeResponseMessage| m.req_id() == &req_id)
                .await?
                .take()
                .take();

            self.handle_key_exchange_local(ctx, m, true).await?;
        } else {
            let responder = key_exchanger.responder();
            let responder = XResponder::new(responder);

            ctx.start_worker(key_exchange_addr, responder).await?;
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let reply = msg.reply().clone();
        let mut onward_route = msg.onward();
        match msg.take() {
            SecureChannelMessage::KeyExchange { payload } => {
                // Received key exchange message from remote channel, need to forward it to local key exchange
                debug!("SecureChannel received KeyExchangeRemote");
                let key_exchange_route;
                if let Some(a) = self.key_exchange_route.clone() {
                    key_exchange_route = a;
                } else {
                    return Err(SecureChannelError::InvalidInternalState.into());
                }

                // Update route to a remote
                self.remote_route = reply;

                // FIXME: Remove req_id in the future when we fix message without length decode
                let req_id = b"CHANNEL_REQ".to_vec();
                ctx.send_message(
                    key_exchange_route,
                    KeyExchangeRequestMessage::Payload {
                        req_id: req_id.clone(),
                        payload,
                    },
                )
                .await?;

                let m = ctx
                    .receive_match(|m: &KeyExchangeResponseMessage| m.req_id() == &req_id)
                    .await?
                    .take()
                    .take();

                self.handle_key_exchange_local(ctx, m, false).await?;
            }
            SecureChannelMessage::Encrypt { m } => {
                debug!("SecureChannel received Encrypt");

                let _ = onward_route.step();

                let msg = TransportMessage {
                    version: 1,
                    onward: onward_route,
                    return_: reply,
                    payload: m,
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

                    let mut vault = self.vault.lock().unwrap();
                    let mut cipher_text = vault.aead_aes_gcm_encrypt(
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

                ctx.send_message(
                    self.remote_route.clone(),
                    SecureChannelMessage::Decrypt { payload },
                )
                .await?;
            }
            SecureChannelMessage::Decrypt { payload } => {
                debug!("SecureChannel received Decrypt");
                let payload = {
                    let keys = Self::get_keys(&mut self.keys)?;

                    if payload.len() < 2 {
                        return Err(SecureChannelError::InvalidNonce.into());
                    }

                    let nonce = Self::convert_nonce_small(&payload.as_slice()[..2])?;

                    let mut vault = self.vault.lock().unwrap();
                    let plain_text = vault.aead_aes_gcm_decrypt(
                        &keys.decrypt_key,
                        &payload[2..],
                        &nonce,
                        &[],
                    )?;

                    plain_text
                };

                let mut transport_message = TransportMessage::decode(&payload)?;

                transport_message
                    .return_
                    .modify()
                    .prepend(self.channel_id.clone());

                ctx.forward_message(transport_message).await?;
            }
        }
        Ok(())
    }
}
