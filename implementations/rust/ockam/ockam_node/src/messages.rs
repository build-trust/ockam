use crate::channel_types::{small_channel, MessageSender, SmallReceiver, SmallSender};
use crate::{
    error::{NodeError, NodeReason, RouterReason, WorkerReason},
    router::SenderPair,
};
use core::{fmt, sync::atomic::AtomicUsize};
use ockam_core::compat::{string::String, sync::Arc, vec::Vec};
use ockam_core::{Address, Error, RelayMessage, Result, TransportType};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker {
        /// The set of addresses in use by this worker
        addrs: Vec<Address>,
        /// Pair of senders to the worker relay (msgs and ctrl)
        senders: SenderPair,
        /// A detached context/ "worker" runs no relay state
        detached: bool,
        /// A mechanism to read channel fill-state for a worker
        mailbox_count: Arc<AtomicUsize>,
        /// Reply channel for command confirmation
        reply: SmallSender<NodeReplyResult>,
    },
    /// Return a list of all worker addresses
    ListWorkers(SmallSender<NodeReplyResult>),
    /// Add an existing address to a cluster
    SetCluster(Address, String, SmallSender<NodeReplyResult>),
    /// Stop an existing worker
    StopWorker(Address, bool, SmallSender<NodeReplyResult>),
    /// Start a new processor
    StartProcessor(Address, SenderPair, SmallSender<NodeReplyResult>),
    /// Stop an existing processor
    StopProcessor(Address, SmallSender<NodeReplyResult>),
    /// Stop the node (and all workers)
    StopNode(ShutdownType, SmallSender<NodeReplyResult>),
    /// Immediately stop the node runtime
    AbortNode,
    /// Let the router know a particular address has stopped
    StopAck(Address),
    /// Request the sender for a worker address
    SenderReq(Address, SmallSender<NodeReplyResult>),
    /// Register a new router for a route id type
    Router(TransportType, Address, SmallSender<NodeReplyResult>),
    /// Message the router to set an address as "ready"
    SetReady(Address),
    /// Check whether an address has been marked as "ready"
    CheckReady(Address, SmallSender<NodeReplyResult>),
}

impl fmt::Display for NodeMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NodeMessage::StartWorker { .. } => write!(f, "StartWorker"),
            NodeMessage::ListWorkers(_) => write!(f, "ListWorkers"),
            NodeMessage::SetCluster(_, _, _) => write!(f, "SetCluster"),
            NodeMessage::StopWorker(_, _, _) => write!(f, "StopWorker"),
            NodeMessage::StartProcessor(_, _, _) => write!(f, "StartProcessor"),
            NodeMessage::StopProcessor(_, _) => write!(f, "StopProcessor"),
            NodeMessage::StopNode(_, _) => write!(f, "StopNode"),
            NodeMessage::AbortNode => write!(f, "AbortNode"),
            NodeMessage::StopAck(_) => write!(f, "StopAck"),
            NodeMessage::SenderReq(_, _) => write!(f, "SenderReq"),
            NodeMessage::Router(_, _, _) => write!(f, "Router"),
            NodeMessage::SetReady(_) => write!(f, "SetReady"),
            NodeMessage::CheckReady(_, _) => write!(f, "CheckReady"),
        }
    }
}

impl NodeMessage {
    /// Create a start worker message
    ///
    /// * `senders`: message and command senders for the relay
    ///
    /// * `detached`: indicate whether this worker address has a full
    ///               relay behind it that can respond to shutdown
    ///               commands.  Setting this to `true` will disable
    ///               stop ACK support in the router
    pub fn start_worker(
        addrs: Vec<Address>,
        senders: SenderPair,
        detached: bool,
        mailbox_count: Arc<AtomicUsize>,
    ) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (reply, rx) = small_channel();
        (
            Self::StartWorker {
                addrs,
                senders,
                detached,
                mailbox_count,
                reply,
            },
            rx,
        )
    }

    /// Create a start worker message
    pub fn start_processor(
        address: Address,
        senders: SenderPair,
    ) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::StartProcessor(address, senders, tx), rx)
    }

    /// Create a stop worker message and reply receiver
    pub fn stop_processor(address: Address) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::StopProcessor(address, tx), rx)
    }

    /// Create a list worker message and reply receiver
    pub fn list_workers() -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::ListWorkers(tx), rx)
    }

    /// Create a set cluster message and reply receiver
    pub fn set_cluster(addr: Address, label: String) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::SetCluster(addr, label, tx), rx)
    }

    /// Create a stop worker message and reply receiver
    pub fn stop_worker(address: Address, detached: bool) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::StopWorker(address, detached, tx), rx)
    }

    /// Create a stop node message
    pub fn stop_node(tt: ShutdownType) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::StopNode(tt, tx), rx)
    }

    /// Create a sender request message and reply receiver
    pub fn sender_request(route: Address) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::SenderReq(route, tx), rx)
    }

    /// Create a SetReady message and reply receiver
    pub fn set_ready(addr: Address) -> Self {
        Self::SetReady(addr)
    }

    /// Create a GetReady message and reply receiver
    pub fn get_ready(addr: Address) -> (Self, SmallReceiver<NodeReplyResult>) {
        let (tx, rx) = small_channel();
        (Self::CheckReady(addr, tx), rx)
    }
}

/// The reply/result of a Node
pub type NodeReplyResult = core::result::Result<RouterReply, Error>;

/// Successful return values from a router command
#[derive(Debug)]
pub enum RouterReply {
    /// Success with no payload
    Ok,
    /// A list of worker addresses
    Workers(Vec<Address>),
    /// Message sender to a specific worker
    Sender {
        /// The address a message is being sent to
        addr: Address,
        /// The relay sender
        sender: MessageSender<RelayMessage>,
    },
    /// Indicate the 'ready' state of an address
    State(bool),
}

/// Specify the type of node shutdown
///
/// For most users `ShutdownType::Graceful()` is recommended.  The
/// `Default` implementation uses a 1 second timeout.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ShutdownType {
    /// Execute a graceful shutdown given a maximum timeout
    ///
    /// The following steps will be taken by the internal router
    /// during graceful shutdown procedure:
    ///
    /// * Signal clusterless workers to stop
    /// * Wait for shutdown ACK hooks from worker set
    /// * Signal worker clusters in reverse-creation order to stop
    /// * Wait for shutdown ACK hooks from each cluster before moving onto the
    ///   next
    /// * All shutdown-signaled workers may process their entire mailbox,
    ///   while not allowing new messages to be queued
    ///
    /// Graceful shutdown procedure will be pre-maturely terminated
    /// when reaching the timeout (failover into `Immediate`
    /// strategy).  **A given timeout of `0` will wait forever!**
    Graceful(u8),
    /// Immediately shutdown workers and run shutdown hooks
    ///
    /// This strategy can lead to data loss:
    ///
    /// * Unhandled mailbox messages will be dropped
    /// * Shutdown hooks may not be able to send messages
    ///
    /// This strategy is not recommended for general use, but will be
    /// selected as a failover, if the `Graceful` strategy reaches its
    /// timeout limit.
    Immediate,
}

impl Default for ShutdownType {
    fn default() -> Self {
        Self::Graceful(1)
    }
}

impl RouterReply {
    /// Return [RouterReply::Ok]
    pub fn ok() -> NodeReplyResult {
        Ok(RouterReply::Ok)
    }

    /// Return [RouterReply::State]
    pub fn state(b: bool) -> NodeReplyResult {
        Ok(RouterReply::State(b))
    }

    /// Return [NodeError::Address] not found
    pub fn no_such_address(a: Address) -> NodeReplyResult {
        Err(NodeError::Address(a).not_found())
    }

    /// Return [NodeError::Address] already exists for the given address
    pub fn worker_exists(a: Address) -> NodeReplyResult {
        Err(NodeError::Address(a).already_exists())
    }

    /// Return [NodeError::RouterState] already exists
    pub fn router_exists() -> NodeReplyResult {
        Err(NodeError::RouterState(RouterReason::Duplicate).already_exists())
    }

    /// Return [NodeError::NodeState] conflict
    pub fn node_rejected(reason: NodeReason) -> NodeReplyResult {
        Err(NodeError::NodeState(reason).conflict())
    }

    /// Return [NodeError::WorkerState] conflict
    pub fn worker_rejected(reason: WorkerReason) -> NodeReplyResult {
        Err(NodeError::WorkerState(reason).conflict())
    }

    /// Return [RouterReply::Workers] for the given addresses
    pub fn workers(v: Vec<Address>) -> NodeReplyResult {
        Ok(Self::Workers(v))
    }

    /// Return [RouterReply::Sender] for the given information
    pub fn sender(addr: Address, sender: MessageSender<RelayMessage>) -> NodeReplyResult {
        Ok(RouterReply::Sender { addr, sender })
    }

    /// Consume the wrapper and return [RouterReply::Sender]
    pub fn take_sender(self) -> Result<(Address, MessageSender<RelayMessage>)> {
        match self {
            Self::Sender { addr, sender } => Ok((addr, sender)),
            _ => Err(NodeError::NodeState(NodeReason::Unknown).internal()),
        }
    }

    /// Consume the wrapper and return [RouterReply::Workers]
    pub fn take_workers(self) -> Result<Vec<Address>> {
        match self {
            Self::Workers(w) => Ok(w),
            _ => Err(NodeError::NodeState(NodeReason::Unknown).internal()),
        }
    }

    /// Consume the wrapper and return [RouterReply::State]
    pub fn take_state(self) -> Result<bool> {
        match self {
            Self::State(b) => Ok(b),
            _ => Err(NodeError::NodeState(NodeReason::Unknown).internal()),
        }
    }

    /// Returns Ok if self is [RouterReply::Ok]
    pub fn is_ok(self) -> Result<()> {
        match self {
            Self::Ok => Ok(()),
            _ => Err(NodeError::NodeState(NodeReason::Unknown).internal()),
        }
    }
}
