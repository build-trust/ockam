use crate::tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::{error::Error, relay::RelayMessage, router::SenderPair};
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, AddressSet};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker {
        /// The set of addresses in use by this worker
        addrs: AddressSet,
        /// Pair of senders to the worker relay (msgs and ctrl)
        senders: SenderPair,
        /// A bare worker runs no relay state
        bare: bool,
        /// Reply channel for command confirmation
        reply: Sender<NodeReplyResult>,
    },
    /// Return a list of all worker addresses
    ListWorkers(Sender<NodeReplyResult>),
    /// Add an existing address to a cluster
    SetCluster(Address, String, Sender<NodeReplyResult>),
    /// Stop an existing worker
    StopWorker(Address, Sender<NodeReplyResult>),
    /// Start a new processor
    StartProcessor(Address, SenderPair, Sender<NodeReplyResult>),
    /// Stop an existing processor
    StopProcessor(Address, Sender<NodeReplyResult>),
    /// Stop the node (and all workers)
    StopNode(ShutdownType, Sender<NodeReplyResult>),
    /// Immediately stop the node runtime
    AbortNode,
    /// Let the router know a particular address has stopped
    StopAck(Address),
    /// Request the sender for a worker address
    SenderReq(Address, Sender<NodeReplyResult>),
    /// Register a new router for a route id type
    Router(u8, Address, Sender<NodeReplyResult>),
    /// Check if a given address is already registered
    CheckAddress(AddressSet, Sender<NodeReplyResult>),
}

impl NodeMessage {
    /// Create a start worker message
    ///
    /// * `senders`: message and command senders for the relay
    ///
    /// * `bare`: indicate whether this worker address has a full
    ///   relay behind it that can respond to shutdown commands.
    ///   Setting this to `true` will disable stop ACK support in the
    ///   router
    pub fn start_worker(
        addrs: AddressSet,
        senders: SenderPair,
        bare: bool,
    ) -> (Self, Receiver<NodeReplyResult>) {
        let (reply, rx) = channel(1);
        (
            Self::StartWorker {
                addrs,
                senders,
                bare,
                reply,
            },
            rx,
        )
    }

    /// Create a start worker message
    pub fn start_processor(
        address: Address,
        senders: SenderPair,
    ) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StartProcessor(address, senders, tx), rx)
    }

    /// Create a stop worker message and reply receiver
    pub fn stop_processor(address: Address) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StopProcessor(address, tx), rx)
    }

    /// Create a list worker message and reply receiver
    pub fn list_workers() -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::ListWorkers(tx), rx)
    }

    /// Create a set cluster message and reply receiver
    pub fn set_cluster(addr: Address, label: String) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::SetCluster(addr, label, tx), rx)
    }

    /// Create a stop worker message and reply receiver
    pub fn stop_worker(address: Address) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StopWorker(address, tx), rx)
    }

    /// Create a stop node message
    pub fn stop_node(tt: ShutdownType) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StopNode(tt, tx), rx)
    }

    /// Create a sender request message and reply receiver
    pub fn sender_request(route: Address) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::SenderReq(route, tx), rx)
    }

    /// Create a message to check the availability of an address set
    pub fn check_address(addrs: AddressSet) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::CheckAddress(addrs, tx), rx)
    }
}

/// The reply/result of a Node
pub type NodeReplyResult = Result<NodeReply, NodeError>;

/// Successful return values from a router command
#[derive(Debug)]
pub enum NodeReply {
    /// Success with no payload
    Ok,
    /// A list of worker addresses
    Workers(Vec<Address>),
    /// Message sender to a specific worker
    Sender {
        /// The address a message is being sent to
        addr: Address,
        /// The relay sender
        sender: Sender<RelayMessage>,
        /// Indicate whether the relay message needs to be constructed
        /// with router wrapping.
        wrap: bool,
    },
}

/// Failure states from a router command
#[derive(Debug)]
pub enum NodeError {
    /// No such worker
    NoSuchWorker(Address),
    /// No such processor
    NoSuchProcessor(Address),
    /// Worker already exists
    WorkerExists(Address),
    /// Router already exists
    RouterExists,
    /// Command rejected
    Rejected(Reason),
}

/// The reason why a command was rejected
#[derive(Debug, Copy, Clone)]
pub enum Reason {
    /// Rejected because the node is currently shutting down
    NodeShutdown,
    /// Rejected because the worker is currently shutting down
    WorkerShutdown,
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
    /// * All shutdown-signalled workers may process their entire mailbox,
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

impl NodeReply {
    /// Return [NodeReply::Ok]
    pub fn ok() -> NodeReplyResult {
        Ok(NodeReply::Ok)
    }

    /// Return [NodeError::NoSuchWorker]
    pub fn no_such_worker(a: Address) -> NodeReplyResult {
        Err(NodeError::NoSuchWorker(a))
    }

    /// Return [NodeError::NoSuchProcessor]
    pub fn no_such_processor(a: Address) -> NodeReplyResult {
        Err(NodeError::NoSuchProcessor(a))
    }

    /// Return [NodeError::WorkerExists] for the given address
    pub fn worker_exists(a: Address) -> NodeReplyResult {
        Err(NodeError::WorkerExists(a))
    }

    /// Return [NodeError::RouterExists]
    pub fn router_exists() -> NodeReplyResult {
        Err(NodeError::RouterExists)
    }

    /// Return [NodeReply::Rejected(reason)]
    pub fn rejected(reason: Reason) -> NodeReplyResult {
        Err(NodeError::Rejected(reason))
    }

    /// Return [NodeReply::Workers] for the given addresses
    pub fn workers(v: Vec<Address>) -> NodeReplyResult {
        Ok(Self::Workers(v))
    }

    /// Return [NodeReply::Sender] for the given information
    pub fn sender(addr: Address, sender: Sender<RelayMessage>, wrap: bool) -> NodeReplyResult {
        Ok(NodeReply::Sender { addr, sender, wrap })
    }

    /// Consume the wrapper and return [NodeReply::Sender]
    pub fn take_sender(self) -> Result<(Address, Sender<RelayMessage>, bool), Error> {
        match self {
            Self::Sender { addr, sender, wrap } => Ok((addr, sender, wrap)),
            _ => Err(Error::InternalIOFailure),
        }
    }

    /// Consume the wrapper and return [NodeReply::Workers]
    pub fn take_workers(self) -> Result<Vec<Address>, Error> {
        match self {
            Self::Workers(w) => Ok(w),
            _ => Err(Error::InternalIOFailure),
        }
    }

    /// Returns Ok if self is [NodeReply::Ok]
    pub fn is_ok(self) -> Result<(), Error> {
        match self {
            Self::Ok => Ok(()),
            _ => Err(Error::InternalIOFailure),
        }
    }
}
