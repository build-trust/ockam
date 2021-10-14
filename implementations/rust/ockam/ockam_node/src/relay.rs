use ockam_core::compat::vec::Vec;
use ockam_core::{Address, Encodable, LocalMessage, Route, RouterMessage};

mod processor_relay;
mod shutdown_handle;
mod worker_relay;

pub use processor_relay::*;
pub use shutdown_handle::*;
pub use worker_relay::*;

/// A message addressed to a relay
#[derive(Clone, Debug)]
pub struct RelayMessage {
    pub addr: Address,
    pub data: RelayPayload,
    pub onward: Route,
}

impl RelayMessage {
    /// Construct a message addressed to a user worker
    pub fn direct(addr: Address, local_msg: LocalMessage, onward: Route) -> Self {
        Self {
            addr,
            data: RelayPayload::Direct(local_msg),
            onward,
        }
    }

    /// Construct a message addressed to an middleware router
    #[inline]
    pub fn pre_router(addr: Address, local_msg: LocalMessage, onward: Route) -> Self {
        let route = local_msg.transport().return_route.clone();
        let r_msg = RouterMessage::Route(local_msg);
        Self {
            addr,
            data: RelayPayload::PreRouter(r_msg.encode().unwrap(), route),
            onward,
        }
    }

    /// Consume this message into its base components
    #[inline]
    pub fn local_msg(self) -> (Address, LocalMessage) {
        (
            self.addr,
            match self.data {
                RelayPayload::Direct(msg) => msg,
                _ => panic!("Called transport() on invalid RelayMessage type!"),
            },
        )
    }
}

#[derive(Clone, Debug)]
pub enum RelayPayload {
    Direct(LocalMessage),
    PreRouter(Vec<u8>, Route),
}
