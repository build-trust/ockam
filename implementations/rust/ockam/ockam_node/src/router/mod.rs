mod record;
mod shutdown;
mod start_processor;
mod start_worker;
mod state;
mod stop_processor;
mod stop_worker;
mod utils;

#[cfg(feature = "metrics")]
use std::sync::atomic::AtomicUsize;

use record::{AddressMeta, AddressRecord, InternalMap};
use state::{NodeState, RouterState};

use crate::channel_types::{router_channel, MessageSender, RouterReceiver, SmallSender};
use crate::{
    error::{NodeError, NodeReason},
    relay::CtrlSignal,
    NodeMessage, NodeReplyResult, RouterReply, ShutdownType,
};
use ockam_core::compat::{collections::BTreeMap, sync::Arc};
use ockam_core::flow_control::FlowControls;
use ockam_core::{Address, RelayMessage, Result, TransportType};

/// A pair of senders to a worker relay
#[derive(Debug)]
pub struct SenderPair {
    pub msgs: MessageSender<RelayMessage>,
    pub ctrl: SmallSender<CtrlSignal>,
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
    receiver: Option<RouterReceiver<NodeMessage>>,
}

enum RouteType {
    Internal,
    External(TransportType),
}

fn determine_type(next: &Address) -> RouteType {
    if next.transport_type().is_local() {
        RouteType::Internal
    } else {
        RouteType::External(next.transport_type())
    }
}

impl Router {
    pub fn new(flow_controls: &FlowControls) -> Self {
        let (sender, receiver) = router_channel();
        Self {
            state: RouterState::new(sender),
            map: InternalMap::new(flow_controls),
            external: BTreeMap::new(),
            receiver: Some(receiver),
        }
    }

    #[cfg(feature = "metrics")]
    pub(crate) fn get_metrics_readout(&self) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        self.map.get_metrics()
    }

    /// Get the router receiver
    fn get_recv(&mut self) -> Result<&mut RouterReceiver<NodeMessage>> {
        self.receiver
            .as_mut()
            .ok_or_else(|| NodeError::NodeState(NodeReason::Corrupt).internal())
    }

    pub fn init(&mut self, addr: Address, senders: SenderPair) {
        self.map.insert_address_record(
            addr.clone(),
            AddressRecord::new(
                vec![addr.clone()],
                senders.msgs,
                senders.ctrl,
                Arc::new(0.into()), // don't track for app worker (yet?)
                AddressMeta {
                    processor: false,
                    detached: true,
                },
            ),
        );
        self.map.insert_alias(&addr, &addr);
    }

    pub fn sender(&self) -> SmallSender<NodeMessage> {
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

    async fn check_addr_not_exist(
        &self,
        addr: &Address,
        reply: &SmallSender<NodeReplyResult>,
    ) -> Result<()> {
        if self.map.address_records_map().contains_key(addr) {
            let node = NodeError::Address(addr.clone());

            reply
                .send(Err(node.clone().already_exists()))
                .await
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;

            Err(node.already_exists())
        } else {
            Ok(())
        }
    }

    async fn handle_msg(&mut self, msg: NodeMessage) -> Result<bool> {
        #[cfg(feature = "metrics")]
        self.map.update_metrics(); // Possibly remove this from the hot path?

        use NodeMessage::*;
        #[cfg(feature = "metrics")]
        trace!(
            "Current router alloc: {} addresses",
            self.map.get_addr_count()
        );
        match msg {
            // Successful router registration command
            Router(tt, addr, sender) if !self.external.contains_key(&tt) => {
                // TODO: Remove after other transport implementations are moved to new architecture
                trace!("Registering new router for type {}", tt);

                self.external.insert(tt, addr);
                sender
                    .send(RouterReply::ok())
                    .await
                    .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?
            }
            // Rejected router registration command
            Router(_, _, sender) => {
                // TODO: Remove after other transport implementations are moved to new architecture
                sender
                    .send(RouterReply::router_exists())
                    .await
                    .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?
            }

            //// ==! Basic worker control
            StartWorker {
                addrs,
                senders,
                detached,
                mailbox_count,
                ref reply,
            } => start_worker::exec(self, addrs, senders, detached, mailbox_count, reply).await?,
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
                // This sets state to stopping, and the sends the AbortNode message
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
                    self.map.clear_address_records_map();
                    return Ok(true);
                }
            }

            StopAck(addr) if self.state.running() => {
                trace!("Received shutdown ACK for address {}", addr);
                self.map.free_address(addr);
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
                    self.map.address_records_map().keys().cloned().collect(),
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
                RouteType::Internal => utils::resolve(self, addr, reply).await?,
                // TODO: Remove after other transport implementations are moved to new architecture
                RouteType::External(tt) => {
                    let addr = utils::router_addr(self, tt)?;
                    utils::resolve(self, &addr, reply).await?
                }
            },
        }

        Ok(false)
    }

    /// Block current task running this router.  Return fatal errors
    async fn run_inner(&mut self) -> Result<()> {
        while let Some(msg) = self.get_recv()?.recv().await {
            let msg_str = format!("{}", msg);
            match self.handle_msg(msg).await {
                Ok(should_break) => {
                    if should_break {
                        // We drop the receiver end here
                        self.receiver.take();
                        break;
                    }
                }
                Err(err) => {
                    debug!("Router error: {} while handling {}", err, msg_str);
                }
            }
        }

        Ok(())
    }
}
