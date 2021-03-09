use crate::{error::Error, relay::RelayMessage};
use ockam_core::{Address, AddressSet, Route};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker(AddressSet, Sender<RelayMessage>),
    /// Return a list of all worker addresses
    ListWorkers(Sender<NodeReplyResult>),
    /// Stop an existing worker
    StopWorker(Address, Sender<NodeReplyResult>),
    /// Stop the node (and all workers)
    StopNode,
    /// Request the sender for an existing worker
    SenderReq(Route, Sender<NodeReplyResult>),
    /// Register a new router for a route id type
    Router(u8, Address, Sender<NodeReplyResult>),
}

impl NodeMessage {
    /// Create a start worker message
    pub fn start_worker(address: AddressSet, sender: Sender<RelayMessage>) -> Self {
        Self::StartWorker(address, sender)
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
    pub fn sender_request(route: Route) -> (Self, Receiver<NodeReplyResult>) {
        let (tx, rx) = channel(1);
        (Self::SenderReq(route, tx), rx)
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
    Sender(Address, Sender<RelayMessage>),
}

/// Failure states from a router command
#[derive(Debug)]
pub enum NodeError {
    NoSuchWorker(Address),
    RouterExists,
}

impl NodeReply {
    pub fn ok() -> NodeReplyResult {
        Ok(NodeReply::Ok)
    }

    pub fn no_such_worker(a: Address) -> NodeReplyResult {
        Err(NodeError::NoSuchWorker(a))
    }

    pub fn router_exists() -> NodeReplyResult {
        Err(NodeError::RouterExists)
    }

    pub fn workers(v: Vec<Address>) -> NodeReplyResult {
        Ok(Self::Workers(v))
    }

    pub fn sender(a: Address, s: Sender<RelayMessage>) -> NodeReplyResult {
        Ok(NodeReply::Sender(a, s))
    }

    pub fn take_sender(self) -> Result<(Address, Sender<RelayMessage>), Error> {
        match self {
            Self::Sender(addr, s) => Ok((addr, s)),
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
