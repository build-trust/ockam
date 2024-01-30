use crate::{Address, LocalMessage, Route};

/// A message addressed to the relay responsible for delivery of the
/// wrapped [`LocalMessage`]
#[derive(Clone, Debug)]
pub struct RelayMessage {
    source: Address,
    destination: Address,
    local_msg: LocalMessage,
}

impl RelayMessage {
    /// Construct a new message addressed to a user worker
    pub fn new(source: Address, destination: Address, local_msg: LocalMessage) -> Self {
        Self {
            source,
            destination,
            local_msg,
        }
    }

    /// The sender address of the wrapped `LocalMessage`
    /// Note that this may be different from the first hop in the return_route
    /// This address is always equal to the address of the `Context` instance used to
    /// send or forward the message
    pub fn source(&self) -> &Address {
        &self.source
    }

    /// The recipient address for the wrapped `LocalMessage`
    /// Note that this may be different from the first hop in the onward_route, for example while
    /// sending a message to an External address (e.g. TCP) first message will be delivered to the
    /// the TCP Router (and destination address will be the address of the TCP Router), and only
    /// then to the individual connection worker
    pub fn destination(&self) -> &Address {
        &self.destination
    }

    /// Onward route
    pub fn onward_route(&self) -> &Route {
        self.local_msg.onward_route_ref()
    }

    /// Return route
    pub fn return_route(&self) -> &Route {
        self.local_msg.return_route_ref()
    }

    /// Payload
    pub fn payload(&self) -> &[u8] {
        self.local_msg.payload_ref()
    }

    /// Local message
    pub fn local_message(&self) -> &LocalMessage {
        &self.local_msg
    }

    /// Take local message
    pub fn into_local_message(self) -> LocalMessage {
        self.local_msg
    }
}
