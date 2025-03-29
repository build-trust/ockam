use crate::compat::collections::HashMap;
use crate::compat::sync::{Arc, RwLock};
use crate::flow_control::{ConsumersInfo, FlowControlId, ProducerInfo};
use crate::Address;

/// Storage for all Flow Control-related data
#[derive(Clone, Debug)]
pub struct FlowControls {
    // All known consumers
    pub(super) consumers: Arc<RwLock<HashMap<FlowControlId, ConsumersInfo>>>,
    // All known producers
    pub(super) producers: Arc<RwLock<HashMap<Address, ProducerInfo>>>,
    // Allows to find producer by having its additional Address,
    // e.g. Decryptor by its Encryptor Address or TCP Receiver by its TCP Sender Address
    pub(super) producers_additional_addresses: Arc<RwLock<HashMap<Address, Address>>>,
    // All known spawners
    pub(super) spawners: Arc<RwLock<HashMap<Address, FlowControlId>>>,
}
