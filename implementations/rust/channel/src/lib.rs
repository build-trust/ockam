#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_extern_crates,
    unused_parens,
    while_true
)]

//! Implements the Ockam channels interface and provides
//! a C FFI version.
//!
//! Channels are where parties can send messages securely

#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[macro_use]
extern crate ockam_common;

use core::marker::PhantomData;
use error::*;
use ockam_common::commands::ockam_commands::{ChannelCommand, OckamCommand, RouterCommand};
use ockam_kex::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_message::message::{Address, Message, MessageType, Route, RouterAddress};
use ockam_vault::DynVault;
use rand::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

/// A Channel Manager creates secure channels on demand using the specified key exchange
/// generic. All keys will be created in the associated vault object
pub struct ChannelManager<
    I: KeyExchanger + 'static,
    R: KeyExchanger + 'static,
    E: NewKeyExchanger<I, R>,
> {
    channels: BTreeMap<String, Channel>,
    receiver: Receiver<OckamCommand>,
    sender: Sender<OckamCommand>,
    router: Sender<OckamCommand>,
    vault: Arc<Mutex<dyn DynVault + Send>>,
    pending_messages: Vec<Message>,
    new_key_exchanger: E,
    phantom_i: PhantomData<I>,
    phantom_r: PhantomData<R>,
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> std::fmt::Debug
    for ChannelManager<I, R, E>
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ChannelManager {{ channels: {:?}, pending_messages: {:?}, receiver, sender, router, vault }}", self.channels, self.pending_messages)
    }
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> ChannelManager<I, R, E> {
    /// Create a new Channel Manager
    pub fn new(
        sender: Sender<OckamCommand>,
        receiver: Receiver<OckamCommand>,
        router: Sender<OckamCommand>,
        vault: Arc<Mutex<dyn DynVault + Send>>,
        new_key_exchanger: E,
    ) -> Self {
        Self {
            channels: BTreeMap::new(),
            sender,
            receiver,
            router,
            vault,
            pending_messages: Vec::new(),
            new_key_exchanger,
            phantom_i: PhantomData,
            phantom_r: PhantomData,
        }
    }

    /// Check for work to be done and do it
    pub fn poll(&mut self) -> Result<bool, ChannelError> {
        let mut keep_going = true;
        loop {
            match self.receiver.try_recv()? {
                OckamCommand::Channel(ChannelCommand::Stop) => {
                    self.channels.clear();
                    self.pending_messages.clear();
                    keep_going = false;
                    break;
                }
                OckamCommand::Channel(ChannelCommand::SendMessage(m)) => {
                    self.handle_send(m)?;
                    break;
                }
                OckamCommand::Channel(ChannelCommand::ReceiveMessage(m)) => {
                    self.handle_recv(m)?;
                    break;
                }
                _ => {}
            }
        }
        // Process pending messages
        let mut set = BTreeSet::new();
        for i in 0..self.pending_messages.len() {
            debug_assert!(!self.pending_messages[i].onward_route.addresses.is_empty());

            let address = self.pending_messages[i].onward_route.addresses[0]
                .address
                .as_string();
            if let Some(channel) = self.channels.get(&address) {
                if channel.completed_key_exchange.is_some() {
                    // Can send now
                    set.insert(i);
                }
            }
        }
        // Send out pending messages
        for i in set.iter().rev() {
            let m = self.pending_messages.remove(*i);
            self.sender
                .send(OckamCommand::Channel(ChannelCommand::SendMessage(m)))?;
        }

        keep_going |= !self.pending_messages.is_empty();

        Ok(keep_going)
    }

    fn handle_recv(&mut self, mut m: Message) -> Result<(), ChannelError> {
        if m.onward_route.addresses.is_empty() {
            // no onward route, how to determine which channel to decrypt message?
            // can't so drop
            return Err(ChannelErrorKind::RecvError.into());
        }
        // If address is zero, it indicates to create a new channel responder for key agreement
        // Otherwise pop the first onward route off to get the channel id
        let address = m.onward_route.addresses[0].address.as_string();
        if address == "0" {
            self.create_new_responder(&m)?;
        } else {
            match self.channels.get_mut(&address) {
                Some(channel) => {
                    match m.message_type {
                        MessageType::KeyAgreementM2 => {
                            let ka_m2 = channel.agreement.process(&m.message_body)?;
                            let m2 = Message {
                                onward_route: m.return_route.clone(),
                                return_route: Route {
                                    addresses: vec![m.onward_route.addresses[0].clone()],
                                },
                                message_type: MessageType::KeyAgreementM3,
                                message_body: ka_m2,
                            };
                            self.router
                                .send(OckamCommand::Router(RouterCommand::SendMessage(m2)))?;
                            if channel.completed_key_exchange.is_none() {
                                // key agreement has finished, now can process any pending messages
                                channel.completed_key_exchange =
                                    Some(channel.agreement.finalize()?);
                            }
                        }
                        MessageType::KeyAgreementM3 => {
                            // For now ignore anything returned from M3
                            let _ = channel.agreement.process(&m.message_body)?;
                            debug_assert!(channel.agreement.is_complete());
                            if channel.completed_key_exchange.is_none() {
                                // key agreement has finished, now can process any pending messages
                                channel.completed_key_exchange =
                                    Some(channel.agreement.finalize()?);
                            }
                        }
                        MessageType::Payload => {
                            // Decrypt, put address on onward route at 0 and send
                            if m.message_body.len() < 2 {
                                return Err(ChannelErrorKind::RecvError.into());
                            }
                            let kex = channel.completed_key_exchange.as_ref().unwrap();
                            m.message_body = {
                                let mut vault = self.vault.lock().unwrap();
                                vault.aead_aes_gcm_decrypt(
                                    kex.decrypt_key,
                                    &m.message_body[2..],
                                    &m.message_body[..2],
                                    &kex.h,
                                )?
                            };
                            m.onward_route.addresses = m.onward_route.addresses[1..].to_vec();
                            self.router
                                .send(OckamCommand::Router(RouterCommand::SendMessage(m)))?;
                        }
                        _ => debug_assert!(false),
                    };
                }
                None => {
                    debug_assert!(false, "unknown channel address");
                    // Do nothing and drop message
                }
            };
        }
        Ok(())
    }

    fn create_new_responder(&mut self, m: &Message) -> Result<(), ChannelError> {
        let mut rng = thread_rng();
        let channel_id = rng.gen::<u32>();
        let channel_address = Address::ChannelAddress(channel_id.to_be_bytes().to_vec());
        let mut channel = Channel::new(channel_id, Box::new(self.new_key_exchanger.responder()));
        self.send_ka_m2(&mut channel, m)?;
        self.channels.insert(channel_address.as_string(), channel);
        Ok(())
    }

    fn send_ka_m2(&mut self, channel: &mut Channel, m: &Message) -> Result<(), ChannelError> {
        let ka_m2 = channel.agreement.process(&m.message_body)?;
        let m2 = Message {
            onward_route: m.return_route.clone(),
            return_route: Route {
                addresses: vec![RouterAddress::from_address(channel.as_address()).unwrap()],
            },
            message_type: MessageType::KeyAgreementM2,
            message_body: ka_m2,
        };
        self.router
            .send(OckamCommand::Router(RouterCommand::SendMessage(m2)))?;
        Ok(())
    }

    fn handle_send(&mut self, m: Message) -> Result<(), ChannelError> {
        if m.onward_route.addresses.is_empty() {
            return Err(ChannelErrorKind::CantSend.into());
        }
        let address = m.onward_route.addresses[0].address.as_string();
        match self.channels.get_mut(&address) {
            Some(channel) => {
                if !channel.agreement.is_complete() {
                    debug_assert!(channel.completed_key_exchange.is_none());
                    // TODO: wait until channel key agreement is finished, what to do with pending
                    // message
                    return Ok(());
                }
                debug_assert!(channel.completed_key_exchange.is_some());
                let cke = channel.completed_key_exchange.as_ref().unwrap();
                let mut vault = self.vault.lock().unwrap();
                let mut ciphertext_and_tag = vault.aead_aes_gcm_encrypt(
                    cke.encrypt_key,
                    &m.message_body,
                    channel.nonce.to_le_bytes().as_ref(),
                    &cke.h,
                )?;
                let mut message_body = channel.nonce.to_le_bytes().to_vec();
                message_body.append(&mut ciphertext_and_tag);
                channel.nonce += 1;
                //TODO: check if key rotation needs to happen

                let mut return_route = m.return_route.clone();
                return_route
                    .addresses
                    .push(m.onward_route.addresses[0].clone());
                let new_m = Message {
                    onward_route: Route {
                        addresses: m.onward_route.addresses[1..].to_vec(),
                    },
                    return_route,
                    message_type: m.message_type,
                    message_body,
                };
                self.router
                    .send(OckamCommand::Router(RouterCommand::SendMessage(new_m)))?;
            }
            None => {
                // Create new channel and start key exchange as initiator
                let mut rng = thread_rng();
                let channel_id = rng.gen::<u32>();
                let channel_zero = Address::ChannelAddress(vec![0u8; 4]);
                let channel_address = Address::ChannelAddress(channel_id.to_be_bytes().to_vec());

                let mut channel =
                    Channel::new(channel_id, Box::new(self.new_key_exchanger.initiator()));
                let ka_m1 = channel.agreement.process(&[])?;

                let mut onward_route = m.onward_route.clone();
                onward_route
                    .addresses
                    .push(RouterAddress::from_address(channel_zero).unwrap());

                let m1 = Message {
                    onward_route,
                    return_route: Route {
                        addresses: vec![RouterAddress::from_address(channel_address).unwrap()],
                    },
                    message_type: MessageType::KeyAgreementM1,
                    message_body: ka_m1,
                };

                self.channels.insert(address, channel);

                // start the key exchange while holding this pending message
                self.router
                    .send(OckamCommand::Router(RouterCommand::SendMessage(m1)))?;

                self.pending_messages.push(m);
            }
        };
        Ok(())
    }
}

struct Channel {
    completed_key_exchange: Option<CompletedKeyExchange>,
    id: u32,
    agreement: Box<dyn KeyExchanger>,
    nonce: u16,
}

impl std::fmt::Debug for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Channel {{ completed_key_exchange: {:?}, id: {:?}, nonce: {:?}, agreement }}",
            self.completed_key_exchange, self.id, self.nonce
        )
    }
}

impl Channel {
    pub fn new(id: u32, agreement: Box<dyn KeyExchanger>) -> Self {
        Self {
            id,
            agreement,
            completed_key_exchange: None,
            nonce: 0,
        }
    }

    pub fn as_address(&self) -> Address {
        Address::ChannelAddress(self.id.to_be_bytes().to_vec())
    }
}

/// Represents the errors that occur within a channel
pub mod error;

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_kex::xx::{XXInitiator, XXNewKeyExchanger, XXResponder};
    use ockam_kex::CipherSuite;
    use ockam_message::message::AddressType;
    use ockam_vault::software::DefaultVault;
    use std::sync::mpsc::channel;

    type XXInitiatorChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;
    type XXResponderChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;

    #[test]
    fn new_channel_initiator() {
        let (tx_router, rx_router) = channel();
        let (tx_channel, rx_channel) = channel();

        let vault = Arc::new(Mutex::new(DefaultVault::default()));
        let new_key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            vault.clone(),
        );

        let mut router = ockam_router::router::Router::new(rx_router);
        let mut channel = XXInitiatorChannelManager::new(
            tx_channel.clone(),
            rx_channel,
            tx_router.clone(),
            vault.clone(),
            new_key_exchanger,
        );

        tx_router
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Channel,
                tx_channel.clone(),
            )))
            .unwrap();

        let message = Message {
            onward_route: Route {
                addresses: vec![RouterAddress::channel_router_address_from_str("deadbeef").unwrap()],
            },
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: b"Hello Bob".to_vec(),
        };

        tx_router
            .send(OckamCommand::Router(RouterCommand::SendMessage(message)))
            .unwrap();
        assert!(router.poll());
        let res = channel.poll();
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn new_channel_responder() {
        let (tx_router, rx_router) = channel();
        let (tx_channel, rx_channel) = channel();

        let vault = Arc::new(Mutex::new(DefaultVault::default()));
        let new_key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            vault.clone(),
        );

        let mut router = ockam_router::router::Router::new(rx_router);
        let mut channel = XXResponderChannelManager::new(
            tx_channel.clone(),
            rx_channel,
            tx_router.clone(),
            vault.clone(),
            new_key_exchanger,
        );

        tx_router
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Channel,
                tx_channel.clone(),
            )))
            .unwrap();

        let message = Message {
            onward_route: Route {
                addresses: vec![RouterAddress::channel_router_address_from_str("00").unwrap()],
            },
            return_route: Route { addresses: vec![] },
            message_type: MessageType::KeyAgreementM1,
            message_body: vec![
                79, 30, 59, 197, 255, 25, 84, 22, 3, 63, 63, 45, 98, 206, 16, 137, 39, 108, 13,
                171, 237, 191, 172, 115, 63, 124, 209, 114, 59, 97, 28, 82,
            ],
        };

        tx_router
            .send(OckamCommand::Router(RouterCommand::SendMessage(message)))
            .unwrap();
        assert!(router.poll());
        let res = channel.poll();
        assert!(res.is_ok());
        assert!(res.unwrap());
    }
}
