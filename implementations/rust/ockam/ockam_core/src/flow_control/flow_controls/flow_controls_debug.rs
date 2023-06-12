use crate::flow_control::FlowControls;
use crate::Address;

impl FlowControls {
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
