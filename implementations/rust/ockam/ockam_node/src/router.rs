use crate::{error::Error, relay::RelayMessage, NodeMessage, NodeReply, NodeReplyResult};
use ockam_core::{Address, AddressSet, Result};
use std::collections::BTreeMap;
use tokio::sync::mpsc::{channel, Receiver, Sender};

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
    /// Primary mapping of worker senders
    internal: BTreeMap<Address, Sender<RelayMessage>>,
    /// Additional address map
    ///
    /// Each worker has a primary address, with secondary addresses
    /// that are stored in this map.  When shutting down a worker,
    /// secondary address senders also need to be cleared for the
    /// worker to shut down
    addr_map: BTreeMap<Address, AddressSet>,
    /// Externally registered router components
    external: BTreeMap<u8, Address>,
    /// Receiver for messages from node
    receiver: Receiver<NodeMessage>,
    /// Keeping a copy of the channel sender to pass out
    sender: Sender<NodeMessage>,
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
            internal: BTreeMap::new(),
            addr_map: BTreeMap::new(),
            external: BTreeMap::new(),
            receiver,
            sender,
        }
    }

    pub fn init(&mut self, addr: Address, mb: Sender<RelayMessage>) {
        self.internal.insert(addr, mb);
    }

    pub fn sender(&self) -> Sender<NodeMessage> {
        self.sender.clone()
    }

    /// Block current task running this router.  Return fatal errors
    pub async fn run(&mut self) -> Result<()> {
        use NodeMessage::*;
        while let Some(mut msg) = self.receiver.recv().await {
            match msg {
                // Internal registration commands
                Router(tt, addr, sender) if !self.external.contains_key(&tt) => {
                    trace!("Registering new router for type {}", tt);

                    self.external.insert(tt, addr);
                    sender
                        .send(NodeReply::ok())
                        .await
                        .map_err(|_| Error::InternalIOFailure)?
                }
                Router(_, _, sender) => sender
                    .send(NodeReply::router_exists())
                    .await
                    .map_err(|_| Error::InternalIOFailure)?,

                // Basic worker control
                StartWorker(addr, sender) => self.start_worker(addr, sender).await?,
                StopWorker(ref addr, ref mut reply) => self.stop_worker(addr, reply).await?,

                // Basic node control
                StopNode => {
                    self.internal.clear();
                    break;
                }
                ListWorkers(sender) => sender
                    .send(NodeReply::workers(self.internal.keys().cloned().collect()))
                    .await
                    .map_err(|_| Error::InternalIOFailure)?,

                // Handle route/ sender requests
                SenderReq(ref addr, ref mut reply) => match determine_type(addr) {
                    RouteType::Internal(ref addr) => self.resolve(addr, reply, false).await?,
                    RouteType::External(tt) => {
                        let addr = self.router_addr(tt)?;
                        self.resolve(&addr, reply, true).await?
                    }
                },
            }
        }

        Ok(())
    }

    async fn start_worker(
        &mut self,
        addrs: AddressSet,
        sender: Sender<RelayMessage>,
    ) -> Result<()> {
        trace!("Starting new worker '{}'", addrs.first());
        addrs.iter().for_each(|addr| {
            self.internal.insert(addr.clone(), sender.clone());
        });
        self.addr_map.insert(addrs.first(), addrs);
        Ok(())
    }

    async fn stop_worker(
        &mut self,
        addr: &Address,
        reply: &mut Sender<NodeReplyResult>,
    ) -> Result<()> {
        trace!("Stopping worker '{}'", addr);

        let addrs = self.addr_map.remove(addr).unwrap();

        match addrs.iter().fold(Some(()), |opt, addr| {
            match (opt, self.internal.remove(addr)) {
                (Some(_), Some(_)) => Some(()),
                (Some(_), None) => None,
                (None, _) => None,
            }
        }) {
            Some(_) => reply.send(NodeReply::ok()),
            None => reply.send(NodeReply::no_such_worker(addr.clone())),
        }
        .await
        .map_err(|_| Error::InternalIOFailure)?;

        Ok(())
    }

    /// Receive an address and resolve it to a sender
    ///
    /// This function only applies to local address types, and will
    /// fail to resolve a correct address if it given a remote
    /// address.
    async fn resolve(
        &mut self,
        addr: &Address,
        reply: &mut Sender<NodeReplyResult>,
        wrap: bool,
    ) -> Result<()> {
        trace!("Resolvivg worker address '{}'", addr);

        match self.internal.get(addr) {
            Some(sender) => reply.send(NodeReply::sender(addr.clone(), sender.clone(), wrap)),
            None => reply.send(NodeReply::no_such_worker(addr.clone())),
        }
        .await
        .expect("Ockam node internal I/O failed!");

        Ok(())
    }

    fn router_addr(&mut self, tt: u8) -> Result<Address> {
        self.external
            .get(&tt)
            .cloned()
            .ok_or(Error::InternalIOFailure.into())
    }
}
