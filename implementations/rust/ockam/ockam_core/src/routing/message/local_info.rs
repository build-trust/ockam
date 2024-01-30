use serde::{Deserialize, Serialize};

use crate::compat::string::String;
use crate::compat::vec::Vec;
use crate::Message;

/// Contains metadata that will only be routed locally within the
/// local Ockam Node.
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct LocalInfo {
    type_identifier: String,
    data: Vec<u8>,
}

impl LocalInfo {
    /// Creates a new `LocalInfo` structure from the provided type identifier and data.
    pub fn new(type_identifier: String, data: Vec<u8>) -> Self {
        LocalInfo {
            type_identifier,
            data,
        }
    }
}

impl LocalInfo {
    /// LocalInfo unique type identifier
    pub fn type_identifier(&self) -> &str {
        &self.type_identifier
    }
    /// LocalInfo raw binary data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
