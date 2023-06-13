use crate::flow_control::{FlowControlId, FlowControls};
use crate::Address;

impl FlowControls {
    fn cleanup_spawner(&self, address: &Address) {
        let spawner_flow_control_id = match self.spawners.write().unwrap().remove(address) {
            None => return, // Wasn't a Spawner
            Some(id) => id,
        };

        // Check if Spawners with the same FlowControlId exist
        let spawners_exist = self
            .spawners
            .read()
            .unwrap()
            .iter()
            .any(|(_addr, flow_control_id)| flow_control_id == &spawner_flow_control_id);

        // Another spawner exists, nothing else we can clean up now
        if spawners_exist {
            return;
        }

        // Check if Producers spawned by this Spawner still exist
        let producers_exist =
            self.producers.read().unwrap().iter().any(|(_addr, info)| {
                match &info.spawner_flow_control_id() {
                    None => false,
                    Some(id) => id == &spawner_flow_control_id,
                }
            });

        // Producers Spawned with this FlowControlId still exist, nothing else we can clean up now
        if producers_exist {
            return;
        }

        // Spawners don't exist, Producers don't exist as well, which means storing Consumers
        // for that FlowControlId doesn't make sense anymore
        self.consumers
            .write()
            .unwrap()
            .remove(&spawner_flow_control_id);
    }

    fn cleanup_producers_spawner(&self, flow_control_id: &FlowControlId) {
        // Check if that Spawner still exists
        let spawner_exists = self
            .spawners
            .read()
            .unwrap()
            .iter()
            .any(|x| x.1 == flow_control_id);

        // Spawner still exists, nothing else we can clean up now
        if spawner_exists {
            return;
        }

        // Check if other Producers spawned by the same Spawner exist
        let other_producers_exist =
            self.producers
                .read()
                .unwrap()
                .iter()
                .any(|x| match x.1.spawner_flow_control_id() {
                    None => false,
                    Some(id) => id == flow_control_id,
                });

        // No other producer exists as well
        if other_producers_exist {
            return;
        }

        // We can clean Consumers for that FlowControlId
        self.consumers.write().unwrap().remove(flow_control_id);
    }

    fn cleanup_producer(&self, address: &Address) {
        let info = match self.producers.write().unwrap().remove(address) {
            None => return, // Wasn't a Producer
            Some(info) => info,
        };

        // Clean known Additional Addresses for that Producer
        self.producers_additional_addresses
            .write()
            .unwrap()
            .retain(|_additional_address, main_address| main_address != address);

        // We have a Spawner
        if let Some(spawner_flow_control_id) = info.spawner_flow_control_id() {
            self.cleanup_producers_spawner(spawner_flow_control_id);
        }

        let flow_control_id = info.flow_control_id;

        // Check if other Producers for the same FlowControlId exist
        let other_producers_exist = self
            .producers
            .read()
            .unwrap()
            .iter()
            .any(|x| x.1.flow_control_id() == &flow_control_id);
        if other_producers_exist {
            return;
        }

        // We can clean Consumers for that FlowControlId
        self.consumers.write().unwrap().remove(&flow_control_id);
    }

    fn cleanup_consumer(&self, address: &Address) {
        // Just remove this Address from Consumers
        let mut consumers = self.consumers.write().unwrap();

        consumers
            .iter_mut()
            .for_each(|(_flow_control_id, info)| _ = info.0.remove(address));

        // Remove empty Maps
        consumers.retain(|_, info| !info.0.is_empty());
        drop(consumers);

        let mut consumers = self.consumers.write().unwrap();

        consumers
            .iter_mut()
            .for_each(|(_flow_control_id, info)| _ = info.0.remove(address));

        // Remove empty Maps
        consumers.retain(|_, info| !info.0.is_empty());
    }

    /// Clean everything that is possible after [`Address`] no longer exists
    pub fn cleanup_address(&self, address: &Address) {
        debug!("Cleanup FlowControls for {address}");

        self.cleanup_spawner(address);
        self.cleanup_producer(address);
        self.cleanup_consumer(address);
    }
}
