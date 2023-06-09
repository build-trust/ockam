use crate::compat::collections::BTreeMap;
use crate::compat::sync::{Arc, RwLock};
use crate::flow_control::{
    ProducerConsumersInfo, ProducerFlowControlId, ProducerInfo, SpawnerConsumersInfo,
    SpawnerFlowControlId,
};
use crate::Address;

/// Storage for all Flow Control-related data
#[derive(Clone, Debug)]
pub struct FlowControls {
    // All known consumers
    pub(super) consumers_for_spawners:
        Arc<RwLock<BTreeMap<SpawnerFlowControlId, SpawnerConsumersInfo>>>,
    pub(super) consumers_for_producers:
        Arc<RwLock<BTreeMap<ProducerFlowControlId, ProducerConsumersInfo>>>,
    // All known producers
    pub(super) producers: Arc<RwLock<BTreeMap<Address, ProducerInfo>>>,
    // Allows to find producer by having its additional Address,
    // e.g. Decryptor by its Encryptor Address or TCP Receiver by its TCP Sender Address
    pub(super) producers_additional_addresses: Arc<RwLock<BTreeMap<Address, Address>>>,
    // All known spawners
    pub(super) spawners: Arc<RwLock<BTreeMap<Address, SpawnerFlowControlId>>>,
}
