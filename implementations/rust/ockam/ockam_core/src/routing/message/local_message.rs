use crate::{compat::vec::Vec, TransportMessage};
use serde::{Deserialize, Serialize};

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
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct LocalMessage {
    transport_message: TransportMessage,
    local_info: Vec<u8>,
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
    pub fn local_info(&self) -> &Vec<u8> {
        &self.local_info
    }
}

impl LocalMessage {
    /// Constructor
    pub fn new(transport_message: TransportMessage, local_info: Vec<u8>) -> Self {
        LocalMessage {
            transport_message,
            local_info,
        }
    }
}
