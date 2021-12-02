use crate::{compat::string::String, compat::vec::Vec, Message, TransportMessage};
use serde::{Deserialize, Serialize};

/// Ockam Routing LocalInfo - metadata that can travel only inside one Ockam Node
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct LocalInfo {
    type_identifier: String,
    data: Vec<u8>,
}

impl LocalInfo {
    /// Constructor
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

/// LocalMessage is a message type that is routed locally within one node.
///
/// LocalMessage consists of TransportMessage + local info in binary format, that can be added by
/// Workers within the same node. TransportMessages are used to transfer messages between
/// different nodes using Transport Workers. Upon arrival to receiving Transport Worker,
/// TransportMessage is wrapped inside LocalMessage and forwarded to other Workers inside that node.
///
/// LocalMessage provides mechanism of transporting metadata that is trusted to come
/// from the same node, which is convenient for delegating Authentication/Authorization mechanisms
/// to dedicated local Workers.
///
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub struct LocalMessage {
    transport_message: TransportMessage,
    local_info: Vec<LocalInfo>,
}

impl LocalMessage {
    /// Underlying transport message
    pub fn into_transport_message(self) -> TransportMessage {
        self.transport_message
    }
    /// Underlying transport message
    pub fn transport(&self) -> &TransportMessage {
        &self.transport_message
    }
    /// Underlying transport message
    pub fn transport_mut(&mut self) -> &mut TransportMessage {
        &mut self.transport_message
    }
    /// LocalInfo added by Workers within the same node
    pub fn local_info(&self) -> &[LocalInfo] {
        &self.local_info
    }
}

impl LocalMessage {
    /// Constructor
    pub fn new(transport_message: TransportMessage, local_info: Vec<LocalInfo>) -> Self {
        LocalMessage {
            transport_message,
            local_info,
        }
    }
}
