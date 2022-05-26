mod record;
mod shutdown;
mod start_processor;
mod start_worker;
mod state;
mod stop_processor;
mod stop_worker;
mod utils;

use record::{AddressMeta, AddressRecord, InternalMap};
use state::{NodeState, RouterState};

use crate::tokio::sync::mpsc::{channel, Receiver, Sender, UnboundedSender};
use crate::{
    error::{NodeError, NodeReason},
    relay::{CtrlSignal, RelayMessage},
    NodeMessage, RouterReply, ShutdownType,
};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Result, TransportType};

/// A pair of senders to a worker relay
#[derive(Debug)]
pub struct SenderPair {
    pub msgs: UnboundedSender<RelayMessage>,
    pub ctrl: Sender<CtrlSignal>,
}

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
    external: BTreeMap<TransportType, Address>,
    /// Receiver for messages from node
    receiver: Receiver<NodeMessage>,
}

enum RouteType {
    Internal(Address),
    External(TransportType),
}

fn determine_type(next: &Address) -> RouteType {
    if next.transport_type().is_local() {
        RouteType::Internal(next.clone())
    } else {
        RouteType::External(next.transport_type())
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

    pub fn init(&mut self, addr: Address, senders: SenderPair) {
        self.map.internal.insert(
            addr.clone(),
            AddressRecord::new(
                addr.clone().into(),
                senders.msgs,
                senders.ctrl,
                AddressMeta {
                    processor: false,
                    detached: true,
                },
            ),
        );
        self.map.addr_map.insert(addr.clone(), addr);
    }

    pub fn sender(&self) -> Sender<NodeMessage> {
        self.state.sender.clone()
    }

    /// A utility facade to hide failures that are not really failures
    pub async fn run(&mut self) -> Result<()> {
        match self.run_inner().await {
            // Everything is A-OK :)
            Ok(()) => Ok(()),
            // If the router has already shut down this failure is a
            // red-herring and should be ignored -- we _have_ just
            // terminated all workers and any message still in the
            // system will crash the whole runtime.
            Err(_) if !self.state.running() => {
                warn!("One (or more) internal I/O failures caused by ungraceful router shutdown!");
                Ok(())
            }
            // If we _are_ still actually running then this is a real
            // failure and needs to be escalated
            e => e,
        }
    }

    async fn handle_msg(&mut self, msg: NodeMessage) -> Result<bool> {
        use NodeMessage::*;
        debug!("Currently router has {} addresses", self.map.count());
        match msg {
            // Successful router registration command
            Router(tt, addr, sender) if !self.external.contains_key(&tt) => {
                trace!("Registering new router for type {}", tt);

                self.external.insert(tt, addr);
                sender
                    .send(RouterReply::ok())
                    .await
                    .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?
            }
            // Rejected router registration command
            Router(_, _, sender) => sender
                .send(RouterReply::router_exists())
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?,

            //// ==! Basic worker control
            StartWorker {
                addrs,
                senders,
                detached,
                ref reply,
            } => start_worker::exec(self, addrs, senders, detached, reply).await?,
            StopWorker(ref addr, ref detached, ref reply) => {
                stop_worker::exec(self, addr, *detached, reply).await?
            }

            //// ==! Basic processor control
            StartProcessor(addr, senders, ref reply) => {
                start_processor::exec(self, addr, senders, reply).await?
            }
            StopProcessor(ref addr, ref reply) => stop_processor::exec(self, addr, reply).await?,

            //// ==! Core node controls
            StopNode(ShutdownType::Graceful(timeout), reply) => {
                if shutdown::graceful(self, timeout, reply).await? {
                    info!("No more workers left.  Goodbye!");
                    if let Some(sender) = self.state.stop_reply() {
                        sender
                            .send(RouterReply::ok())
                            .await
                            .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
                        return Ok(true);
                    };
                }
            }
            StopNode(ShutdownType::Immediate, reply) => {
                shutdown::immediate(self, reply).await?;
                return Ok(true);
            }

            AbortNode => {
                if let Some(sender) = self.state.stop_reply() {
                    sender
                        .send(RouterReply::ok())
                        .await
                        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
                    self.map.internal.clear();
                    return Ok(true);
                }
            }

            StopAck(addr) if self.state.running() => {
                debug!("Received shutdown ACK for address {}", addr);
                if let Some(rec) = self.map.internal.remove(&addr) {
                    rec.address_set().iter().for_each(|addr| {
                        self.map.addr_map.remove(addr);
                    });
                }
            }

            StopAck(addr) => {
                if shutdown::ack(self, addr).await? {
                    info!("No more workers left.  Goodbye!");
                    if let Some(sender) = self.state.stop_reply() {
                        sender
                            .send(RouterReply::ok())
                            .await
                            .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
                        return Ok(true);
                    }
                }
            }

            ListWorkers(sender) => sender
                .send(RouterReply::workers(
                    self.map.internal.keys().cloned().collect(),
                ))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?,

            SetCluster(addr, label, reply) => {
                debug!("Setting cluster on address {}", addr);
                let msg = self.map.set_cluster(label, addr);
                reply
                    .send(msg)
                    .await
                    .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
            }

            SetReady(addr) => {
                trace!("Marking address {} as ready!", addr);
                match self.map.set_ready(addr) {
                    Err(e) => warn!("Failed to set address as ready: {}", e),
                    Ok(waiting) => {
                        for sender in waiting {
                            sender.send(RouterReply::ok()).await.map_err(|_| {
                                NodeError::NodeState(NodeReason::Unknown).internal()
                            })?;
                        }
                    }
                }
            }

            CheckReady(addr, reply) => {
                let ready = self.map.get_ready(addr, reply.clone());
                if ready {
                    reply
                        .send(RouterReply::ok())
                        .await
                        .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
                }
            }

            // Handle route/ sender requests
            SenderReq(ref addr, ref reply) => match determine_type(addr) {
                RouteType::Internal(ref addr) => utils::resolve(self, addr, reply, false).await?,
                RouteType::External(tt) => {
                    let addr = utils::router_addr(self, tt)?;
                    utils::resolve(self, &addr, reply, true).await?
                }
            },
        }

        Ok(false)
    }

    /// Block current task running this router.  Return fatal errors
    async fn run_inner(&mut self) -> Result<()> {
        while let Some(msg) = self.receiver.recv().await {
            let msg_str = format!("{}", msg);
            match self.handle_msg(msg).await {
                Ok(should_break) => {
                    if should_break {
                        break;
                    }
                }
                Err(err) => {
                    warn!("Router error: {} while handling {}", err, msg_str);
                }
            }
        }

        Ok(())
    }
}
