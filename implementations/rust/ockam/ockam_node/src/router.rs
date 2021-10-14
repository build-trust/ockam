// use crate::relay::ShutdownHandle;
use crate::tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::{
    error::Error,
    relay::{RelayMessage, PROC_ADDR_SUFFIX},
    AddressRecord, NodeMessage, NodeReply, NodeReplyResult,
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
    // TODO: Should we have separate registry for Workers and for Processors?
    /// Registry that maps primary address to all Addresses of a Worker and its Sender
    internal: BTreeMap<Address, AddressRecord>,
    /// Registry of primary addresses for all known addresses
    addr_map: BTreeMap<Address, Address>,
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
        self.internal
            .insert(addr.clone(), AddressRecord::new(addr.clone().into(), mb));
        self.addr_map.insert(addr.clone(), addr);
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

                // Stop node will stop all workers and processors by
                // dropping their sending channels.
                StopNode => {
                    self.internal.clear();
                    break;
                }
                ListWorkers(sender) => sender
                    .send(NodeReply::workers(self.internal.keys().cloned().collect()))
                    .await
                    .map_err(|_| Error::InternalIOFailure)?,
                StartProcessor(addr, main_sender, aux_sender, ref reply) => {
                    self.start_processor(addr.into(), main_sender, aux_sender, reply)

                        .await?
                }
                StopProcessor(ref addr, ref reply) => self.stop_processor(addr, reply).await?,

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

        let primary_addr = addrs.first();

        let address_record = AddressRecord::new(addrs.clone(), sender);

        self.internal.insert(primary_addr.clone(), address_record);

        #[cfg(feature = "std")]
        if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
            trace!("{:#?}", self.internal);
        }
        #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
        trace!("{:#?}", self.internal);

        addrs.iter().for_each(|addr| {
            self.addr_map.insert(addr.clone(), primary_addr.clone());
        });

        // For now we just send an OK back -- in the future we need to
        // communicate the current executor state
        reply
            .send(NodeReply::ok())
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        Ok(())
    }

    async fn start_processor(
        &mut self,
        addr: Address,
        main_sender: Sender<RelayMessage>,
        aux_sender: Sender<RelayMessage>,
        reply: &Sender<NodeReplyResult>,
    ) -> Result<()> {
        trace!("Starting new processor '{}'", &addr);

        let aux_addr = addr.suffix(PROC_ADDR_SUFFIX);

        let main_record = AddressRecord::new(addr.clone().into(), main_sender);
        let aux_record = AddressRecord::new(aux_addr.clone().into(), aux_sender);

        // We insert both records without a reference to each other
        // because when we stop the processor we can easily derive the
        // aux address via the well-known suffix.  If this is at some
        // point no longer sufficient, we should start adding the
        // addresses as full aliases via address-record or addr_map
        self.internal.insert(addr.clone(), main_record);
        self.internal.insert(aux_addr.clone(), aux_record);

        #[cfg(feature = "std")]
        if std::env::var("OCKAM_DUMP_INTERNALS").is_ok() {
            trace!("{:#?}", self.internal);
        }
        #[cfg(all(not(feature = "std"), feature = "dump_internals"))]
        trace!("{:#?}", self.internal);

        self.addr_map.insert(addr.clone(), addr.clone());

        // For now we just send an OK back -- in the future we need to
        // communicate the current executor state
        reply
            .send(NodeReply::ok())
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        Ok(())
    }

    async fn stop_processor(
        &mut self,
        main_addr: &Address,
        reply: &Sender<NodeReplyResult>,
    ) -> Result<()> {
        trace!("Stopping processor '{}'", main_addr);

        let aux_addr = main_addr.suffix(PROC_ADDR_SUFFIX);

        // First check if a processor of this address exists and
        // remove both address records.  We can drop both records here
        // too.  For the main address this means that no more messages
        // can be sent to this processor, and for the aux address this
        // initiates processor shutdown.
        match (
            self.internal.remove(&aux_addr),
            self.internal.remove(main_addr),
        ) {
            (Some(_), Some(_)) => {}
            // If by any chance only one of the records existed we are
            // in an undefined router state and will panic (for now)
            (Some(_), None) | (None, Some(_)) => {
                panic!("Invalid router state: mismatching processor address records!")
            }
            _ => {
                reply
                    .send(NodeReply::no_such_processor(main_addr.clone()))
                    .await
                    .map_err(|_| Error::InternalIOFailure)?;
                return Ok(());
            }
        };

        // Remove  main address from addr_map too
        self.addr_map.remove(main_addr);

        // Signal back that everything went OK
        reply
            .send(NodeReply::ok())
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        Ok(())
    }

    async fn stop_worker(&mut self, addr: &Address, reply: &Sender<NodeReplyResult>) -> Result<()> {
        trace!("Stopping worker '{}'", addr);

        let primary_address;
        if let Some(p) = self.addr_map.get(addr) {
            primary_address = p.clone();
        } else {
            reply
                .send(NodeReply::no_such_worker(addr.clone()))
                .await
                .map_err(|_| Error::InternalIOFailure)?;

            return Ok(());
        }

        let record;
        if let Some(r) = self.internal.remove(&primary_address) {
            record = r;
        } else {
            // Actually should not happen
            reply
                .send(NodeReply::no_such_worker(addr.clone()))
                .await
                .map_err(|_| Error::InternalIOFailure)?;

            return Ok(());
        }

        for addr in record.address_set().iter() {
            self.addr_map.remove(addr);
        }

        reply
            .send(NodeReply::ok())
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
        trace!("Resolving worker address '{}'", addr);

        let primary_address;
        if let Some(p) = self.addr_map.get(addr) {
            primary_address = p.clone();
        } else {
            reply
                .send(NodeReply::no_such_worker(addr.clone()))
                .await
                .map_err(|_| Error::InternalIOFailure)?;

            return Ok(());
        }

        match self.internal.get(&primary_address) {
            Some(record) => reply.send(NodeReply::sender(addr.clone(), record.sender(), wrap)),
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
