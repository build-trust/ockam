use crate::{Address, LocalMessage, Route};

/// A message addressed to the relay responsible for delivery of the
/// wrapped [`LocalMessage`]
#[derive(Clone, Debug)]
pub struct RelayMessage {
    /// The sender address of the wrapped `LocalMessage`
    pub source: Address,
    /// The recipient address for the wrapped `LocalMessage`
    pub destination: Address,
    /// The wrapped `LocalMessage`
    pub local_msg: LocalMessage,
    /// The onward route of the wrapped `LocalMessage`
    pub onward: Route,
}

impl RelayMessage {
    /// Construct a new message addressed to a user worker
    pub fn new(
        origin: Address,
        destination: Address,
        local_msg: LocalMessage,
        onward: Route,
    ) -> Self {
        Self {
            source: origin,
            destination,
            local_msg,
            onward,
        }
    }
}
