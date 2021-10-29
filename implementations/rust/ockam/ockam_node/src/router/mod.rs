#![allow(unused)]

mod record;
mod start_processor;
mod start_worker;
mod state;
mod stop_processor;
mod stop_worker;
mod utils;

use record::{InternalMap, AddressRecord, AddressState};
use state::{NodeState, RouterState};

use crate::tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::{
    error::Error,
    relay::{RelayMessage, PROC_ADDR_SUFFIX},
    NodeMessage, NodeReply, NodeReplyResult,
};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, AddressSet, Result};

/// A combined address type and local worker router
///
/// This router supports two routing modes: internal, and external.
///
/// Internal routing resolves `type=0` addresses to local workers.
///
/// External routing is supported only after a plugin component
/// registers itself with this router.  Only one router can be
/// registered per address type.
pub struct Router {
    /// Keep track of some additional router state information
    state: RouterState,
    /// Internal address state
    map: InternalMap,
    /// Externally registered router components
    external: BTreeMap<u8, Address>,
    /// Receiver for messages from node
    receiver: Receiver<NodeMessage>,
}

enum RouteType {
    Internal(Address),
    External(u8),
}

fn determine_type(next: &Address) -> RouteType {
    if next.tt == 0 {
        RouteType::Internal(next.clone())
    } else {
        RouteType::External(next.tt)
    }
}

impl Router {
    pub fn new() -> Self {
        let (sender, receiver) = channel(32);
        Self {
            state: RouterState::new(sender),
            map: InternalMap::default(),
            external: BTreeMap::new(),
            receiver,
        }
    }

    pub fn init(&mut self, addr: Address, mb: Sender<RelayMessage>) {
        self.map
            .internal.insert(addr.clone(), AddressRecord::new(addr.clone().into(), mb));
        self.map.addr_map.insert(addr.clone(), addr);
    }

    pub fn sender(&self) -> Sender<NodeMessage> {
        self.state.sender.clone()
    }

    /// Block current task running this router.  Return fatal errors
    pub async fn run(&mut self) -> Result<()> {
        use NodeMessage::*;
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                // Successful router registration command
                Router(tt, addr, sender) if !self.external.contains_key(&tt) => {
                    trace!("Registering new router for type {}", tt);

                    self.external.insert(tt, addr);
                    sender
                        .send(NodeReply::ok())
                        .await
                        .map_err(|_| Error::InternalIOFailure)?
                }
                // Rejected router registration command
                Router(_, _, sender) => sender
                    .send(NodeReply::router_exists())
                    .await
                    .map_err(|_| Error::InternalIOFailure)?,

                //// ==! Basic worker control
                StartWorker(addr, sender, ref reply) => {
                    start_worker::exec(self, addr, sender, reply).await?
                }
                StopWorker(ref addr, ref reply) => stop_worker::exec(self, addr, reply).await?,

                //// ==! Basic processor control
                StartProcessor(addr, main_sender, aux_sender, ref reply) => {
                    start_processor::exec(self, addr, main_sender, aux_sender, reply).await?
                }
                StopProcessor(ref addr, ref reply) => {
                    stop_processor::exec(self, addr, reply).await?
                }

                //// ==! Core node controls

                // Check whether a set of addresses is available
                CheckAddress(ref addrs, ref reply) => {
                    utils::check_addr_collisions(self, addrs, reply).await?
                }

                // Stop node will stop all workers and processors by
                // dropping their sending channels.
                StopNode => {
                    self.map.internal.clear();
                    break;
                }
                ListWorkers(sender) => sender
                    .send(NodeReply::workers(self.map.internal.keys().cloned().collect()))
                    .await
                    .map_err(|_| Error::InternalIOFailure)?,

                // Handle route/ sender requests
                SenderReq(ref addr, ref reply) => match determine_type(addr) {
                    RouteType::Internal(ref addr) => {
                        utils::resolve(self, addr, reply, false).await?
                    }
                    RouteType::External(tt) => {
                        let addr = utils::router_addr(self, tt)?;
                        utils::resolve(self, &addr, reply, true).await?
                    }
                },
            }
        }

        Ok(())
    }
}
