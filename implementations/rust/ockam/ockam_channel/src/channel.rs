use crate::channels_facade::{ChannelsFacadeMessage, CHANNELS_FACADE_ADDRESS};
use crate::initiator::XInitiator;
use crate::responder::XResponder;
use crate::KeyExchangeMessage;
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_router::{
    RouteTransportMessage, RouterAddress, TransportMessage, ROUTER_ADDRESS_TYPE_LOCAL,
};
use ockam_transport_tcp::TCP_ROUTER_ADDRESS;
use ockam_vault::SoftwareVault;
use ockam_vault_core::Secret;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

struct ChannelKeys {
    h: [u8; 32],
    encrypt_key: Secret,
    decrypt_key: Secret,
}

pub struct Channel {
    _key_exchange_factory_address: Address,
    transport_address: RouterAddress,
    is_initiator: bool,
    channel_id: String,
    addr_for_key_exchange: Address, // this address is used to receive messages from key exchange worker
    key_exchange_addr: Option<Address>, // this address is used to send messages to key exchange worker
    keys: Option<ChannelKeys>,
    vault: Option<Arc<Mutex<SoftwareVault>>>,
}

impl Channel {
    pub fn new(
        _key_exchange_factory_address: Address,
        transport_address: RouterAddress,
        is_initiator: bool,
        channel_id: String,
        addr_for_key_exchange: Address,
    ) -> Self {
        Channel {
            _key_exchange_factory_address,
            transport_address,
            is_initiator,
            channel_id,
            addr_for_key_exchange,
            key_exchange_addr: None,
            keys: None,
            vault: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelMessage {
    Forward(Vec<u8>),
    InitiationMessage(Vec<u8>),
    KeyExchangeMessage(Vec<u8>),
    ExchangeComplete {
        h: [u8; 32],
        encrypt_key: usize,
        decrypt_key: usize,
    },
    Encrypt(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PlainTextWorkerMessage(Vec<u8>);

#[async_trait]
impl Worker for Channel {
    type Message = ChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> Result<()> {
        // Replace key_exchanger with worker
        let vault = Arc::new(Mutex::new(SoftwareVault::new()));
        self.vault = Some(vault.clone());
        let key_exchanger = XXNewKeyExchanger::new(vault.clone(), vault);

        if self.is_initiator {
            let initiator = key_exchanger.initiator();
            let initiator = XInitiator::new(initiator, self.addr_for_key_exchange.clone());

            let initiator_address: Address = format!("xx_responder/{}", self.channel_id)
                .as_bytes()
                .to_vec()
                .into();

            self.key_exchange_addr = Some(initiator_address.clone());

            context.start_worker(initiator_address, initiator).await
        } else {
            let responder = key_exchanger.responder();
            let responder = XResponder::new(responder, self.addr_for_key_exchange.clone());

            let responder_address: Address = format!("xx_responder/{}", self.channel_id)
                .as_bytes()
                .to_vec()
                .into();

            self.key_exchange_addr = Some(responder_address.clone());

            context.start_worker(responder_address, responder).await
        }
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            ChannelMessage::InitiationMessage(payload) => {
                // TODO: Check who's the sender?
                // Send to network
                let mut m = TransportMessage::new();

                let msg = ChannelsFacadeMessage::RequestNewChannel {
                    channel_id: self.channel_id.clone(),
                    payload,
                };

                // First message goes to message factory
                m.payload = serde_bare::to_vec(&msg).unwrap();
                m.onward_route.addrs = vec![
                    self.transport_address.clone().into(),
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_TYPE_LOCAL,
                        address: CHANNELS_FACADE_ADDRESS.into(),
                    },
                ];

                ctx.send_message(TCP_ROUTER_ADDRESS, RouteTransportMessage::Route(m))
                    .await
            }
            ChannelMessage::KeyExchangeMessage(payload) => {
                // TODO: Check who's the sender?
                // Send to network
                let mut m = TransportMessage::new();

                let msg = ChannelsFacadeMessage::Forward {
                    channel_id: self.channel_id.clone(),
                    payload,
                };

                m.payload = serde_bare::to_vec(&msg).unwrap();

                m.onward_route.addrs = vec![
                    self.transport_address.clone().into(),
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_TYPE_LOCAL,
                        address: CHANNELS_FACADE_ADDRESS.into(),
                    },
                ];

                ctx.send_message(TCP_ROUTER_ADDRESS, RouteTransportMessage::Route(m))
                    .await
            }
            ChannelMessage::ExchangeComplete {
                h,
                encrypt_key,
                decrypt_key,
            } => {
                println!("Channel with id {} completed kex", self.channel_id);
                self.keys = Some(ChannelKeys {
                    h,
                    encrypt_key: Secret::new(encrypt_key),
                    decrypt_key: Secret::new(decrypt_key),
                });

                Ok(())
            }
            ChannelMessage::Forward(payload) => {
                if let Some(_keys) = &self.keys {
                    // Decrypt message and send to some worker
                    unimplemented!()
                // let plain_text = {
                //     let vault = self.vault.as_ref().unwrap();
                //     let mut vault = vault.lock().unwrap();
                //
                //     // FIXME
                //     vault.aead_aes_gcm_decrypt(&keys.decrypt_key, payload.as_slice(), &[], &[])?
                // };
                // ctx.send_message(self.plain_text_worker_addr.clone(), PlainTextWorkerMessage(plain_text)).await
                } else {
                    // Key exchange hasn't completed yet
                    ctx.send_message(
                        self.key_exchange_addr.clone().unwrap(),
                        KeyExchangeMessage::ChannelMessage(payload),
                    )
                    .await
                }
            }
            ChannelMessage::Encrypt(_payload) => {
                unimplemented!()
            }
        };
    }
}
