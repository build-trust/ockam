use crate::compat::rand::random;
use crate::compat::vec::Vec;
use crate::flow_control::{ConsumersInfo, FlowControlId, FlowControls, ProducerInfo};
use crate::Address;

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
    pub fn generate_flow_control_id() -> FlowControlId {
        random()
    }

    /// Mark that given [`Address`] is a Consumer for a Producer with the given [`FlowControlId`]
    pub fn add_consumer(&self, address: &Address, flow_control_id: &FlowControlId) {
        let mut consumers = self.consumers.write().unwrap();
        if !consumers.contains_key(flow_control_id) {
            consumers.insert(flow_control_id.clone(), Default::default());
        }

        let flow_control_consumers = consumers.get_mut(flow_control_id).unwrap();

        if flow_control_consumers.0.insert(address.clone()) {
            debug!("Add Consumer {address} to Producer {flow_control_id}");
        }
    }

    /// Mark that given [`Address`] is a Producer for to the given [`FlowControlId`]
    /// Also, mark that this Producer was spawned by a Spawner
    /// with the given spawner [`FlowControlId`] (if that's the case).
    pub fn add_producer(
        &self,
        address: &Address,
        flow_control_id: &FlowControlId,
        spawner_flow_control_id: Option<&FlowControlId>,
        additional_addresses: Vec<Address>,
    ) {
        let mut producers = self.producers.write().unwrap();
        let is_new = producers
            .insert(
                address.clone(),
                ProducerInfo {
                    flow_control_id: flow_control_id.clone(),
                    spawner_flow_control_id: spawner_flow_control_id.cloned(),
                },
            )
            .is_none();
        drop(producers);

        if is_new {
            debug!(
                "Add Producer {address} with additional_addresses {:?} to {flow_control_id} with spawner {:?}",
                additional_addresses,
                spawner_flow_control_id
            );
        }

        let mut producers_additional_addresses =
            self.producers_additional_addresses.write().unwrap();
        producers_additional_addresses.insert(address.clone(), address.clone());
        for additional_address in additional_addresses {
            producers_additional_addresses.insert(additional_address, address.clone());
        }
    }

    /// Mark that given [`Address`] is a Spawner for to the given [`FlowControlId`]
    pub fn add_spawner(&self, address: &Address, flow_control_id: &FlowControlId) {
        let mut spawners = self.spawners.write().unwrap();

        if spawners
            .insert(address.clone(), flow_control_id.clone())
            .is_none()
        {
            debug!("Add Spawner {address} with {flow_control_id}");
        }
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

    /// Get [`ProducerInfo`] for which given [`Address`] is a Producer
    pub fn get_flow_control_with_producer(&self, address: &Address) -> Option<ProducerInfo> {
        let producers = self.producers.read().unwrap();
        producers.get(address).cloned()
    }

    /// Get [`ProducerInfo`] for which given [`Address`] is a Producer or is an additional [`Address`]
    /// for that Producer (e.g. Encryptor address for its Decryptor, or TCP Sender for its TCP Receiver)
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

    /// Get all [`FlowControlId`]s for which given Address is a consumer
    /// TODO: Optimize
    pub fn get_flow_control_ids_for_consumer(&self, address: &Address) -> Vec<FlowControlId> {
        let consumers = self.consumers.read().unwrap();

        consumers
            .iter()
            .filter_map(|(id, info)| {
                if info.0.contains(address) {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
