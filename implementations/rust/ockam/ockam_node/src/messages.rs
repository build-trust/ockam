use crate::{error::Error, relay::RelayMessage};
use ockam_core::{Address, AddressSet};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker(AddressSet, Sender<RelayMessage>, Sender<NodeReplyResult>),
    /// Return a list of all worker addresses
    ListWorkers(Sender<NodeReplyResult>),
    /// Stop an existing worker
    StopWorker(Address, Sender<NodeReplyResult>),
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
    NoSuchWorker(Address),
    WorkerExists(Address),
    RouterExists,
}

impl NodeReply {
    pub fn ok() -> NodeReplyResult {
        Ok(NodeReply::Ok)
    }

    pub fn no_such_worker(a: Address) -> NodeReplyResult {
        Err(NodeError::NoSuchWorker(a))
    }

    pub fn worker_exists(a: Address) -> NodeReplyResult {
        Err(NodeError::WorkerExists(a))
    }

    pub fn router_exists() -> NodeReplyResult {
        Err(NodeError::RouterExists)
    }

    pub fn workers(v: Vec<Address>) -> NodeReplyResult {
        Ok(Self::Workers(v))
    }

    pub fn sender(addr: Address, sender: Sender<RelayMessage>, wrap: bool) -> NodeReplyResult {
        Ok(NodeReply::Sender { addr, sender, wrap })
    }

    pub fn take_sender(self) -> Result<(Address, Sender<RelayMessage>, bool), Error> {
        match self {
            Self::Sender { addr, sender, wrap } => Ok((addr, sender, wrap)),
            _ => Err(Error::InternalIOFailure.into()),
        }
    }

    pub fn take_workers(self) -> Result<Vec<Address>, Error> {
        match self {
            Self::Workers(w) => Ok(w),
            _ => Err(Error::InternalIOFailure.into()),
        }
    }

    pub fn is_ok(self) -> Result<(), Error> {
        match self {
            Self::Ok => Ok(()),
            _ => Err(Error::InternalIOFailure.into()),
        }
    }
}
