use crate::relay::RelayMessage;
use ockam_core::{Address, AddressSet};
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Messages sent from the Node to the Executor
#[derive(Debug)]
pub enum NodeMessage {
    /// Start a new worker and store the send handle
    StartWorker(AddressSet, Sender<RelayMessage>),
    /// Return a list of all worker addresses
    ListWorkers(Sender<NodeReply>),
    /// Stop an existing worker
    StopWorker(Address, Sender<NodeReply>),
    /// Stop the node (and all workers)
    StopNode,
    /// Request the sender for an existing worker
    SenderReq(Address, Sender<NodeReply>),
}

impl NodeMessage {
    /// Create a start worker message
    pub fn start_worker(address: AddressSet, sender: Sender<RelayMessage>) -> Self {
        Self::StartWorker(address, sender)
    }

    /// Create a list worker message and reply receiver
    pub fn list_workers() -> (Self, Receiver<NodeReply>) {
        let (tx, rx) = channel(1);
        (Self::ListWorkers(tx), rx)
    }

    /// Create a stop worker message and reply receiver
    pub fn stop_worker(address: Address) -> (Self, Receiver<NodeReply>) {
        let (tx, rx) = channel(1);
        (Self::StopWorker(address, tx), rx)
    }

    /// Create a stop node message
    pub fn stop_node() -> Self {
        Self::StopNode
    }

    /// Create a sender request message and reply receiver
    pub fn sender_request(address: Address) -> (Self, Receiver<NodeReply>) {
        let (tx, rx) = channel(1);
        (Self::SenderReq(address, tx), rx)
    }
}

/// Return value of some executor commands
#[derive(Debug)]
pub enum NodeReply {
    /// Everything went ok
    Ok,
    /// Worker address not found
    NoSuchWorker(Address),
    /// A list of worker addresses
    Workers(Vec<Address>),
    /// Message sender to a specific worker
    Sender(Address, Sender<RelayMessage>),
}

impl NodeReply {
    pub fn ok() -> Self {
        Self::Ok
    }

    pub fn no_such_worker(address: Address) -> Self {
        Self::NoSuchWorker(address)
    }

    pub fn sender(address: Address, sender: Sender<RelayMessage>) -> Self {
        Self::Sender(address, sender)
    }
}
