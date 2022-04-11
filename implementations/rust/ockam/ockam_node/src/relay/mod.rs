use ockam_core::{Address, LocalMessage, Route};

mod processor_relay;
mod worker_relay;

pub use processor_relay::*;
pub use worker_relay::*;

/// A message addressed to a relay
#[derive(Clone, Debug)]
pub struct RelayMessage {
    pub addr: Address,
    pub local_msg: LocalMessage,
    pub onward: Route,
    pub needs_wrapping: bool,
}

impl RelayMessage {
    /// Construct a message addressed to a user worker
    pub fn new(
        addr: Address,
        local_msg: LocalMessage,
        onward: Route,
        needs_wrapping: bool,
    ) -> Self {
        Self {
            addr,
            local_msg,
            onward,
            needs_wrapping,
        }
    }
}

/// A signal type used to communicate between router and worker relay
#[derive(Clone, Debug)]
pub enum CtrlSignal {
    /// Interrupt current message execution but resume run-loop
    Interrupt,
    /// Interrupt current message execution and shut down
    InterruptStop,
}
