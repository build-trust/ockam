use crate::{compat::string::String, compat::vec::Vec, Message, TransportMessage};
use serde::{Deserialize, Serialize};

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

/// A message type that is routed locally within a single node.
///
/// `LocalMessage` consists of a [`TransportMessage`] and
/// [`LocalInfo`] in binary format, that can be added by Workers
/// within the same node.
///
/// Transport Messages are used to transfer messages between different
/// nodes using Transport Workers. Upon arrival at a receiving
/// Transport Worker, `TransportMessage` is wrapped inside
/// `LocalMessage` and forwarded to other Workers inside that node.
///
/// `LocalMessage` provides a mechanism for transporting metadata that
/// is trusted to come from the same node. This is convenient for
/// delegating Authentication/Authorization mechanisms to dedicated
/// local Workers.
///
/// This type is exposed in `ockam_core` (and the root `ockam` crate) in
/// order to provide a mechanism for third-party developers to create
/// custom transport channel routers.
///
/// Casual users of Ockam should never have to interact with this type
/// directly.
///
/// # Examples
///
/// See `ockam_transport_tcp::workers::receiver::TcpRecvProcessor` for a usage example.
///
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct LocalMessage {
    transport_message: TransportMessage,
    local_info: Vec<LocalInfo>,
}

impl LocalMessage {
    /// Consumes the message and returns the underlying transport message.
    pub fn into_transport_message(self) -> TransportMessage {
        self.transport_message
    }
    /// Return a reference to the underlying transport message.
    pub fn transport(&self) -> &TransportMessage {
        &self.transport_message
    }
    /// Return a mutable reference to the underlying transport message.
    pub fn transport_mut(&mut self) -> &mut TransportMessage {
        &mut self.transport_message
    }
    /// Return a reference to local information added by Workers within the same node.
    pub fn local_info(&self) -> &[LocalInfo] {
        &self.local_info
    }
    /// Dissolve
    pub fn dissolve(self) -> (TransportMessage, Vec<LocalInfo>) {
        (self.transport_message, self.local_info)
    }
}

impl LocalMessage {
    /// Append a new [`LocalInfo`] entry.
    pub fn append_local_info(&mut self, local_info: LocalInfo) {
        self.local_info.push(local_info)
    }

    /// Replace all [`LocalInfo`] entries matching the type identifier
    /// of the given `LocalInfo` with itself.
    pub fn replace_local_info(&mut self, local_info: LocalInfo) {
        self.clear_local_info(local_info.type_identifier());
        self.local_info.push(local_info)
    }

    /// Clear all [`LocalInfo`] entries with the given type identifier.
    pub fn clear_local_info(&mut self, type_identifier: &str) {
        self.local_info
            .retain(|x| x.type_identifier() != type_identifier)
    }
}

impl LocalMessage {
    /// Create a new `LocalMessage` from the provided transport message and local information.
    pub fn new(transport_message: TransportMessage, local_info: Vec<LocalInfo>) -> Self {
        LocalMessage {
            transport_message,
            local_info,
        }
    }
}
