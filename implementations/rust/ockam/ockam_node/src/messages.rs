// use crate::relay::ShutdownHandle;
use crate::tokio::sync::mpsc::{channel, Receiver, Sender};
use crate::{error::Error, relay::RelayMessage};
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, AddressSet};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker(AddressSet, Sender<RelayMessage>, Sender<NodeReplyResult>),
    /// Return a list of all worker addresses
    ListWorkers(Sender<NodeReplyResult>),
    /// Stop an existing worker
    StopWorker(Address, Sender<NodeReplyResult>),
    /// Start a new processor and store
    StartProcessor(
        Address,
        Sender<RelayMessage>,
        Sender<RelayMessage>,
        Sender<NodeReplyResult>,
    ),
    /// Stop an existing processor
    StopProcessor(Address, Sender<NodeReplyResult>),
    /// Stop the node (and all workers)
    StopNode,
    /// Request the sender for a worker address
    SenderReq(Address, Sender<NodeReplyResult>),
    /// Register a new router for a route id type
    Router(u8, Address, Sender<NodeReplyResult>),
    /// Check if a given address is already registered
    CheckAddress(AddressSet, Sender<NodeReplyResult>),
}

impl NodeMessage {
    /// Create a start worker message
    pub fn start_worker(
        address: AddressSet,
        sender: Sender<RelayMessage>,
    ) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StartWorker(address, sender, tx), rx)
    }

    /// Create a start worker message
    pub fn start_processor(
        address: Address,
        main_sender: Sender<RelayMessage>,
        aux_sender: Sender<RelayMessage>,
    ) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (
            Self::StartProcessor(address, main_sender, aux_sender, tx),
            rx,
        )
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

    /// Create a stop worker message and reply receiver
    pub fn stop_worker(address: Address) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::StopWorker(address, tx), rx)
    }

    /// Create a stop node message
    pub fn stop_node() -> Self {
        Self::StopNode
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
