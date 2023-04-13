use crate::compat::collections::BTreeMap;
use crate::compat::rand::random;
use crate::compat::sync::{Arc, RwLock};
use crate::compat::vec::Vec;
use crate::flow_control::{FlowControlId, FlowControlPolicy};
use crate::Address;

// TODO: Consider integrating this into Routing for better UX + to allow removing
// entries from that storage
/// Storage for all Flow Control-related data
#[derive(Clone, Debug)]
pub struct FlowControls {
    // All known consumers
    consumers: Arc<RwLock<BTreeMap<FlowControlId, ConsumersInfo>>>,
    // All known producers
    producers: Arc<RwLock<BTreeMap<Address, ProducerInfo>>>,
    // Allows to find producer by having its additional Address,
    // e.g. Decryptor by its Encryptor Address or TCP Receiver by its TCP Sender Address
    producers_additional_addresses: Arc<RwLock<BTreeMap<Address, Address>>>,
    // All known spawners
    spawners: Arc<RwLock<BTreeMap<Address, FlowControlId>>>,
}

impl FlowControls {
    /// Constructor
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            consumers: Default::default(),
            producers: Default::default(),
            producers_additional_addresses: Default::default(),
            spawners: Default::default(),
        }
    }
}

impl FlowControls {
    /// Generate a fresh random [`FlowControlId`]
    pub fn generate_id() -> FlowControlId {
        random()
    }

    /// Mark that given [`Address`] is a Consumer for Producer or Spawner with the given [`FlowControlId`]
    pub fn add_consumer(
        &self,
        address: impl Into<Address>,
        flow_control_id: &FlowControlId,
        policy: FlowControlPolicy,
    ) {
        let address = address.into();
        let mut consumers = self.consumers.write().unwrap();
        if !consumers.contains_key(flow_control_id) {
            consumers.insert(flow_control_id.clone(), Default::default());
        }

        let flow_control_consumers = consumers.get_mut(flow_control_id).unwrap();

        flow_control_consumers.0.insert(address, policy);
    }

    /// Mark that given [`Address`] is a Producer for to the given [`FlowControlId`]
    /// Also, mark that this Producer was spawned by a Spawner
    /// with the given spawner [`FlowControlId`] (if that's the case).
    pub fn add_producer(
        &self,
        address: impl Into<Address>,
        flow_control_id: &FlowControlId,
        spawner_flow_control_id: Option<&FlowControlId>,
        additional_addresses: Vec<Address>,
    ) {
        let address = address.into();
        let mut producers = self.producers.write().unwrap();
        producers.insert(
            address.clone(),
            ProducerInfo {
                flow_control_id: flow_control_id.clone(),
                spawner_flow_control_id: spawner_flow_control_id.cloned(),
            },
        );
        drop(producers);

        let mut producers_additional_addresses =
            self.producers_additional_addresses.write().unwrap();
        producers_additional_addresses.insert(address.clone(), address.clone());
        for additional_address in additional_addresses {
            producers_additional_addresses.insert(additional_address, address.clone());
        }
    }

    /// Mark that given [`Address`] is a Spawner for to the given [`FlowControlId`]
    pub fn add_spawner(&self, address: impl Into<Address>, flow_control_id: &FlowControlId) {
        let address = address.into();
        let mut spawners = self.spawners.write().unwrap();

        spawners.insert(address, flow_control_id.clone());
    }

    /// Get known Consumers for the given [`FlowControlId`]
    pub fn get_consumers_info(&self, flow_control_id: &FlowControlId) -> ConsumersInfo {
        let consumers = self.consumers.read().unwrap();
        consumers.get(flow_control_id).cloned().unwrap_or_default()
    }

    /// Get [`FlowControlId`] for which given [`Address`] is a Spawner
    pub fn get_flow_control_with_spawner(&self, address: &Address) -> Option<FlowControlId> {
        let spawners = self.spawners.read().unwrap();
        spawners.get(address).cloned()
    }

    /// Get [`FlowControlId`] for which given [`Address`] is a Producer
    pub fn get_flow_control_with_producer(&self, address: &Address) -> Option<ProducerInfo> {
        let producers = self.producers.read().unwrap();
        producers.get(address).cloned()
    }

    /// Get [`FlowControlId`] for which given [`Address`] is a Producer or is an additional [`Address`]
    /// fot that Producer (e.g. Encryptor address for its Decryptor, or TCP Sender for its TCP Receiver)
    pub fn find_flow_control_with_producer_address(
        &self,
        address: &Address,
    ) -> Option<ProducerInfo> {
        let producers_additional_addresses = self.producers_additional_addresses.read().unwrap();
        let producer_address = match producers_additional_addresses.get(address) {
            Some(address) => address.clone(),
            None => return None,
        };
        drop(producers_additional_addresses);
        let producers = self.producers.read().unwrap();
        producers.get(&producer_address).cloned()
    }

    /// Get all [`FlowControlId`]s for which given [`Address`] is a Consumer
    pub fn get_flow_controls_with_consumer(&self, address: &Address) -> Vec<FlowControlId> {
        let consumers = self.consumers.read().unwrap();
        consumers
            .iter()
            .filter(|&x| x.1 .0.contains_key(address))
            .map(|x| x.0.clone())
            .collect()
    }

    /// Prints debug information regarding Flow Control for the provided address
    #[allow(dead_code)]
    pub fn debug_address(&self, address: &Address) {
        debug!("Address: {}", address.address());
        let consumers = self.get_flow_controls_with_consumer(address);
        if consumers.is_empty() {
            debug!("    No consumers found");
        }
        for consumer in consumers {
            debug!("    Consumer: {:?}", consumer);
        }

        let producers = self.get_flow_control_with_producer(address);
        if producers.is_none() {
            debug!("    No producer found");
        }
        if let Some(producer) = producers {
            debug!("    Producer: {:?}", producer);
        }

        let producers = self.find_flow_control_with_producer_address(address);
        if producers.is_none() {
            debug!("    No producer alias found");
        }
        if let Some(producer) = producers {
            debug!("    Alias Producer: {:?}", producer);
        }
    }
}

/// Known Consumers for the given [`FlowControlId`]
#[derive(Default, Clone, Debug)]
pub struct ConsumersInfo(pub(super) BTreeMap<Address, FlowControlPolicy>);

/// Producer information
#[derive(Clone, Debug)]
pub struct ProducerInfo {
    flow_control_id: FlowControlId,
    spawner_flow_control_id: Option<FlowControlId>,
}

impl ProducerInfo {
    /// [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }

    /// Spawner's [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> &Option<FlowControlId> {
        &self.spawner_flow_control_id
    }
}
