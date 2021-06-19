use crate::{error::Error, relay::RelayMessage, NodeMessage, NodeReply, NodeReplyResult};
use ockam_core::{Address, AddressSet, Result};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::mpsc::{channel, Receiver, Sender};

type RelaySender = Arc<Sender<RelayMessage>>;

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
    ///
    /// For each address a worker listens to a reference to the same
    /// sender will be store in this map, as to allow alias addresses
    /// to be checked for collisions.
    internal: BTreeMap<Address, RelaySender>,
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
        self.internal.insert(addr.clone(), Arc::new(mb));
        self.addr_map.insert(addr.clone(), addr.into());
    }

    pub fn sender(&self) -> Sender<NodeMessage> {
        self.sender.clone()
    }

    /// Block current task running this router.  Return fatal errors
    pub async fn run(&mut self) -> Result<()> {
        use NodeMessage::*;
        while let Some(msg) = self.receiver.recv().await {
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
                StartWorker(addr, sender, ref reply) => {
                    self.start_worker(addr, sender, reply).await?
                }
                StopWorker(ref addr, ref reply) => self.stop_worker(addr, reply).await?,

                // Check whether a set of addresses is available
                CheckAddress(ref addrs, ref reply) => {
                    self.check_addr_collisions(addrs, reply).await?
                }

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
                SenderReq(ref addr, ref reply) => match determine_type(addr) {
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
        reply: &Sender<NodeReplyResult>,
    ) -> Result<()> {
        trace!("Starting new worker '{}'", addrs.first());

        if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
            trace!("{:#?}", self.internal);
        }

        let sender = Arc::new(sender);
        addrs.iter().for_each(|addr| {
            self.internal.insert(addr.clone(), Arc::clone(&sender));
        });
        self.addr_map.insert(addrs.first(), addrs);

        // For now we just send an OK back -- in the future we need to
        // communicate the current executor state
        reply
            .send(NodeReply::ok())
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        Ok(())
    }

    async fn stop_worker(&mut self, addr: &Address, reply: &Sender<NodeReplyResult>) -> Result<()> {
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
        reply: &Sender<NodeReplyResult>,
        wrap: bool,
    ) -> Result<()> {
        trace!("Resolvivg worker address '{}'", addr);

        match self.internal.get(addr) {
            Some(sender) => reply.send(NodeReply::sender(
                addr.clone(),
                sender.as_ref().clone(),
                wrap,
            )),
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
            .ok_or_else(|| Error::InternalIOFailure.into())
    }

    /// Check if an address is already in-use by another worker
    async fn check_addr_collisions(
        &self,
        addrs: &AddressSet,
        reply: &Sender<NodeReplyResult>,
    ) -> Result<()> {
        if let Some(addr) = addrs.iter().fold(None, |acc, addr| {
            match (acc, self.internal.contains_key(addr)) {
                (None, true) => Some(addr.clone()),
                (None, false) => None,
                // If a collision was already found, ignore further collisions
                (Some(addr), _) => Some(addr),
            }
        }) {
            reply.send(NodeReply::worker_exists(addr))
        } else {
            reply.send(NodeReply::ok())
        }
        .await
        .map_err(|_| Error::InternalIOFailure.into())
    }
}
