//! Advanced Ockam worker protocols
//!
//! This module primarily contains types and parsing utilities used in
//! different high-level Ockam utility protocols

use crate::{Message, Result};
use ockam_core::{compat::vec::Vec, ProtocolId};
use serde::{Deserialize, Serialize};

pub mod pipe;
pub mod stream;

/// A protocol payload wrapper for pre-parsing
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct ProtocolPayload {
    pub protocol: ProtocolId,
    pub data: Vec<u8>,
}

impl ProtocolPayload {
    /// Take an encodable message type and wrap it into a protocol payload
    ///
    /// ## Decoding payloads
    ///
    /// In order to easily decode incoming `ProtocolPayload`s, it is
    /// recommended to use the `ProtocolParser` abstraction, which handles
    /// matching between different decoders based on the protocol ID.
    pub fn new<P: Into<ProtocolId>, S: Message>(p: P, d: S) -> Self {
        Self {
            protocol: p.into(),
            data: d.encode().expect("Failed to serialise protocol payload"),
        }
    }
}

/// Map a `ProtocolPayload` to a protocol specific type
///
/// This trait should be implemented for the facade enum-type of a
/// protocol, meaning that the usage will look something like this.
///
/// ```no_compile
/// async fn handle_message(&mut self, _: &mut Context, msg: Routed<Any>) -> Result<()> {
///     let protocol_payload = ProtocolPayload::decode(msg.payload())?;
///     let resp = MyProtocolResponse::parse(protocol_payload)?;
///     println!("{:?}", resp);
///     Ok(())
/// }
/// ```
pub trait ProtocolParser: Sized {
    /// A function which checks whether this parser should be called
    /// for a particular protocol payload.
    ///
    /// Internally it's recommended to use static strings and a set
    /// operation to speed up repeated queries.
    fn check_id(id: &str) -> bool;
    fn parse(pp: ProtocolPayload) -> Result<Self>;
}
