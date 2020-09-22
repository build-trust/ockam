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

use error::*;
use ockam_kex::{error::KeyExchangeFailErrorKind, CompletedKeyExchange, KeyExchanger, DynKeyExchanger};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    convert::TryFrom,
    fmt::{self, Debug, Display},
    io::{Read, Write},
    ops::FnMut,
    sync::mpsc
};
use ockam_vault::DynVault;

/// The template for the sender that Channels use
pub type ChannelSender = mpsc::Sender<usize>;
/// The template for the receiver that Channels use
pub type ChannelReceiver = mpsc::Receiver<usize>;

/// A closure method that is called when channels are handled
pub trait ChannelHandler {
    /// handle this message
    fn handle(&self);
}

pub trait ChannelRekeyHandler {
    fn should_rekey(&self) -> bool;
    fn rekey(&self);
}

/// The Manager for the various channels and registered handler closures
pub struct ChannelManager {
    /// The current channels that are handled
    channels: RefCell<BTreeMap<Connection, Channel>>,
    sender: ChannelSender,
    receiver: mpsc::Receiver<usize>,
    router: Vec<mpsc::Sender<usize>>
}

impl ChannelManager {
    /// Create a new Channel Manager
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            channels: RefCell::new(BTreeMap::new()),
            sender,
            receiver,
            router: Vec::new()
        }
    }

    pub fn register_new_route(&self, onward_route: Vec<usize>, tx_router: mpsc::Sender<usize>) {

    }

    pub fn get_tx(&self) -> ChannelSender {
        self.sender.clone()
    }
}

impl Debug for ChannelManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ChannelManager: {{ channels: {:?}, handler_count: {} }}", self.channels.borrow(), self.handlers.borrow().len())
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
struct Connection {
    return_route: usize,
    onward_route: usize
}

/// Represents an Ockam channel for reading and writing payloads
pub struct Channel {
    exchange_data: Option<CompletedKeyExchange>,
    key_exchanger: Box<dyn DynKeyExchanger + 'static>,
    local_address: usize,
    remote_address: usize,
    vault: Box<dyn DynValue + 'static>,
    rekey: Box<dyn ChannelRekeyHandler + 'static>,
}

impl Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel {{ exchange_data: {:?}, key_exchanger, local_address: {}, remote_address: {} }}",
               self.exchange_data,
               self.local_address,
               self.remote_address)
    }
}

/// The type of addresses that can be handled by `Channel`
#[derive(Copy, Clone, Debug)]
pub enum ChannelAddress {
    /// The address that signals to decrypt the message
    Local,
    /// The address that signals to encrypt the message
    Remote,
    /// The address that signals this message is for the `Channel` itself
    KeyExchangeMessage
}

impl TryFrom<usize> for ChannelAddress {
    type Error = ChannelError;

    fn try_from(data: usize) -> Result<Self, Self::Error> {
        match data {
            1 => Ok(Self::Local),
            2 => Ok(Self::Remote),
            3 => Ok(Self::KeyExchangeMessage),
            _ => Err(ChannelErrorKind::GeneralError { msg: "Unknown Channel Address".to_string() })
        }
    }
}

impl Channel {
    /// Create a new channel with the specified  and `key_exchanger` method
    pub fn new<K: KeyExchanger + Sync + Send + 'static>(key_exchanger: K, local_address: usize, remote_address: usize) -> Self {
        Self {
            exchange_data: None,
            key_exchanger: Box::new(key_exchanger),
            local_address,
            remote_address
        }
    }

    // /// perform is the key exchange as needed to secure the channel
    // pub fn key_exchange<B: AsRef<[u8]>, R: Read, W: Write>(&mut self, message: B, reader: &mut R, writer: &mut W) -> Result<(), ChannelError> {
    //     if self.key_exchanger.is_complete() {
    //         self.exchange_data = Some(self.key_exchanger.finalize()?);
    //     } else {
    //         let message = message.as_ref();
    //         let mut msg = self.key_exchanger.process(message)?;
    //         while !self.key_exchanger.is_complete() {
    //             let written = writer.write(&msg)?;
    //             if written != msg.len() {
    //                 return Err(ChannelErrorKind::KeyAgreement(
    //                     KeyExchangeFailErrorKind::InvalidByteCount(written, msg.len()),
    //                 )
    //                 .into());
    //             }
    //
    //             let mut buffer = Vec::new();
    //             let read = reader.read_to_end(&mut buffer)?;
    //             msg = self.key_exchanger.process(&buffer[..read])?;
    //         }
    //         self.exchange_data = Some(self.key_exchanger.finalize()?);
    //     }
    //     Ok(())
    // }

    pub fn encrypt<B: AsRef<[u8]>>(&self, message: B) -> Result<Vec<u8>, ChannelError> {
        if self.exchange_data.is_none() {
            return Err(ChannelErrorKind::State.into());
        }

        Ok()
    }

    // pub fn decrypt(&self) -> Result<(), ChannelError> {
    //     if self.exchange_data.is_none() {
    //         return Err(ChannelErrorKind::State.into());
    //     }
    //     Ok(())
    // }
}

/// Represents the errors that occur within a channel
pub mod error;
