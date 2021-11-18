//! Ockam pipe protocol structures

pub mod internal;

use crate::Message;
use ockam_core::{Decodable, Encodable, Result, TransportMessage, Uint};
use serde::{Deserialize, Serialize};

/// An indexed message for pipes
#[derive(Serialize, Deserialize, Message)]
pub struct PipeMessage {
    /// Pipe message index
    pub index: Uint,
    /// Pipe message raw data
    pub data: Vec<u8>,
}

impl PipeMessage {
    /// We need to manually implement clone because serde_bare::Uint
    /// doesn't, so we can't derive it
    pub(crate) fn clone(&self) -> Self {
        Self {
            index: Uint::from(self.index.u64()),
            data: self.data.clone(),
        }
    }

    pub(crate) fn from_transport(index: u64, msg: TransportMessage) -> Result<Self> {
        let data = msg.encode()?;
        Ok(Self {
            index: index.into(),
            data,
        })
    }

    pub(crate) fn to_transport(&self) -> Result<TransportMessage> {
        TransportMessage::decode(&self.data)
    }
}
