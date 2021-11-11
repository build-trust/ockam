//! Ockam pipe protocol structures

use ockam_core::Uint;
use serde::{Deserialize, Serialize};

/// An indexed message for pipes
#[derive(Serialize, Deserialize)]
pub struct PipeMessage {
    /// Pipe message index
    pub index: Uint,
    /// Pipe message raw data
    pub data: Vec<u8>,
}
