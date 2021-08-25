//! Advanced Ockam worker protocols

use ockam_core::compat::vec::Vec;
use ockam_core::{Message, ProtocolId};
use serde::{Deserialize, Serialize};

mod parser;
pub use parser::*;

pub mod stream;

/// A protocol payload wrapper for pre-parsing
#[derive(Debug, Serialize, Deserialize)]
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
