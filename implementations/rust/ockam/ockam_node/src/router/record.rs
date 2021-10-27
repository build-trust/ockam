use crate::relay::RelayMessage;
use crate::tokio::sync::mpsc::Sender;
use ockam_core::AddressSet;

#[derive(Debug)]
pub struct AddressRecord {
    address_set: AddressSet,
    sender: Sender<RelayMessage>,
}

impl AddressRecord {
    pub fn address_set(&self) -> &AddressSet {
        &self.address_set
    }
    pub fn sender(&self) -> Sender<RelayMessage> {
        self.sender.clone()
    }
}

impl AddressRecord {
    pub fn new(address_set: AddressSet, sender: Sender<RelayMessage>) -> Self {
        AddressRecord {
            address_set,
            sender,
        }
    }
}

/// Encode the run states a worker or processor can be in
///
/// * Starting - the runner was started and is running `initialize()`
/// * Running - the runner is looping in its main body (either
///   handling messages or a manual run-loop)
/// * Stopping - the runner was signalled to shut-down (running `shutdown()`)
/// * Stopped - the runner has stopped and is in-accessible
/// * Faulty - the runner has experienced an error and is waiting for
///   supervisor intervention
#[derive(Debug)]
#[allow(unused)]
pub enum AddressState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Faulty,
}
