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
use ockam_kex::{error::KeyExchangeFailErrorKind, CompletedKeyExchange, KeyExchanger};
use std::io::{Read, Write};

/// Represents an Ockam channel for reading and writing payloads
#[derive(Debug)]
pub struct Channel<R: Read, W: Write, KX: KeyExchanger> {
    exchange_data: Option<CompletedKeyExchange>,
    key_exchanger: KX,
    reader: R,
    writer: W,
}

impl<R: Read, W: Write, KX: KeyExchanger> Channel<R, W, KX> {
    /// Create a new channel with the specified `reader`, `writer`, and `key_exchanger` method
    pub fn new(key_exchanger: KX, reader: R, writer: W) -> Self {
        Self {
            exchange_data: None,
            key_exchanger,
            reader,
            writer,
        }
    }

    /// perform is the key exchange as needed to secure the channel
    pub fn key_exchange<B: AsRef<[u8]>>(&mut self, message: B) -> Result<(), ChannelError> {
        if self.key_exchanger.is_complete() {
            self.exchange_data = Some(self.key_exchanger.finalize()?);
        } else {
            let message = message.as_ref();
            let mut msg = self.key_exchanger.process(message)?;
            while !self.key_exchanger.is_complete() {
                let written = self.writer.write(&msg)?;
                if written != msg.len() {
                    return Err(ChannelErrorKind::KeyAgreement(
                        KeyExchangeFailErrorKind::InvalidByteCount(written, msg.len()),
                    )
                    .into());
                }

                let mut buffer = Vec::new();
                let read = self.reader.read_to_end(&mut buffer)?;
                msg = self.key_exchanger.process(&buffer[..read])?;
            }
            self.exchange_data = Some(self.key_exchanger.finalize()?);
        }
        Ok(())
    }
}

/// Represents the errors that occur within a channel
pub mod error;
