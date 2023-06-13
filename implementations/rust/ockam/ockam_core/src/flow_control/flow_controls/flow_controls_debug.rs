use crate::compat::vec::Vec;
use crate::flow_control::{FlowControlId, FlowControls};
use crate::Address;
use core::fmt;
use core::fmt::Formatter;

struct IdsCollection(Vec<FlowControlId>);

impl IdsCollection {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for IdsCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.is_empty() {
            write!(f, "[]")?;
        }
        if let Some(first) = self.0.first() {
            write!(f, "{}", first)?;
        }
        self.0.iter().skip(1).fold(Ok(()), |result, id| {
            result.and_then(|_| write!(f, ", {}", id))
        })
    }
}

impl FlowControls {
    fn get_flow_controls_with_consumer(&self, address: &Address) -> IdsCollection {
        IdsCollection(
            self.consumers
                .read()
                .unwrap()
                .iter()
                .filter_map(|(id, info)| {
                    if info.contains(address) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    /// Prints debug information regarding Flow Control for the provided address
    fn debug_address(&self, address: &Address) {
        let consumers = self.get_flow_controls_with_consumer(address);
        if consumers.is_empty() {
            debug!("    No consumers found");
        } else {
            debug!("    Consumers: {}", consumers);
        }

        if let Some(producer) = self.get_flow_control_with_producer(address) {
            debug!("    Producer: {:?}", producer);
        } else {
            debug!("    No producer found");
        }

        if let Some(producer) = self.find_flow_control_with_producer_address(address) {
            debug!("    Alias Producer: {:?}", producer);
        } else {
            debug!("    No producer alias found");
        }
    }

    /// Prints debug information to investigate why message was not allowed to pass through
    pub fn debug_denied_message(
        &self,
        source: &Address,
        source_flow_control_id: &FlowControlId,
        source_spawner_flow_control_id: &Option<FlowControlId>,
        destination: &Address,
    ) {
        warn!("Message was not allowed from {source} to {destination}");
        warn!(
            "  Source: FlowControlId={}, Spawner={:?}",
            source_flow_control_id, source_spawner_flow_control_id
        );
        self.debug_address(source);

        let ids = self.get_flow_controls_with_consumer(destination);
        warn!("  Destination: Consumer FlowControlIds: {}", &ids);
        self.debug_address(destination);
    }
}
