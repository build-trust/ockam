use crate::relay::RelayMessage;
use ockam_core::AddressSet;
use tokio::sync::mpsc::Sender;

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
