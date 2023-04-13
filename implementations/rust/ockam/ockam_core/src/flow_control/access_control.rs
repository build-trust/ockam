use crate::compat::boxed::Box;
use crate::compat::collections::BTreeSet;
use crate::compat::sync::RwLock;
use crate::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use crate::{async_trait, Address, Result};
use crate::{OutgoingAccessControl, RelayMessage};
use core::fmt::{Debug, Formatter};

/// Flow Control Outgoing Access Control
///
/// Allows to send messages only to members of the given [`FlowControlId`] or message a Spawner
/// with given [`FlowControlId`]. Optionally, only 1 message can be passed to the Spawner.
pub struct FlowControlOutgoingAccessControl {
    flow_controls: FlowControls,
    flow_control_id: FlowControlId,
    spawner_flow_control_id: Option<FlowControlId>,
    sent_single_message_to_addresses: RwLock<BTreeSet<Address>>,
}

impl Debug for FlowControlOutgoingAccessControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FlowControlOutgoingAccessControl")
            .field("flow_control_id", &self.flow_control_id)
            .field("spawner_flow_control_id", &self.spawner_flow_control_id)
            .finish()
    }
}

impl FlowControlOutgoingAccessControl {
    /// Constructor
    pub fn new(
        flow_controls: &FlowControls,
        flow_control_id: FlowControlId,
        spawner_flow_control_id: Option<FlowControlId>,
    ) -> Self {
        Self {
            flow_controls: flow_controls.clone(),
            flow_control_id,
            spawner_flow_control_id,
            sent_single_message_to_addresses: Default::default(),
        }
    }
}

#[async_trait]
impl OutgoingAccessControl for FlowControlOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        let next = onward_route.next()?;

        let consumers_info = self.flow_controls.get_consumers_info(&self.flow_control_id);

        if let Some(policy) = consumers_info.0.get(next) {
            match policy {
                FlowControlPolicy::ProducerAllowMultiple => {
                    return crate::allow();
                }
                FlowControlPolicy::SpawnerAllowOnlyOneMessage => {}
                FlowControlPolicy::SpawnerAllowMultipleMessages => {}
            }
        }

        if let Some(spawner_flow_control_id) = &self.spawner_flow_control_id {
            let consumers_info = self
                .flow_controls
                .get_consumers_info(spawner_flow_control_id);

            if let Some(policy) = consumers_info.0.get(next) {
                match policy {
                    FlowControlPolicy::SpawnerAllowOnlyOneMessage => {
                        // We haven't yet sent a message to this address
                        if !self
                            .sent_single_message_to_addresses
                            .read()
                            .unwrap()
                            .contains(next)
                        {
                            // Prevent further messages to this address
                            self.sent_single_message_to_addresses
                                .write()
                                .unwrap()
                                .insert(next.clone());

                            // Allow this message
                            return crate::allow();
                        }
                    }
                    FlowControlPolicy::SpawnerAllowMultipleMessages => return crate::allow(),
                    FlowControlPolicy::ProducerAllowMultiple => {}
                }
            }
        }

        #[cfg(feature = "debugger")]
        {
            self.flow_controls.debug_address(relay_msg.source());
            self.flow_controls.debug_address(next);
        }
        crate::deny()
    }
}
