use crate::compat::boxed::Box;
use crate::flow_control::{FlowControlId, FlowControls};
use crate::{async_trait, Result};
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
        }
    }
}

#[async_trait]
impl OutgoingAccessControl for FlowControlOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        let next = onward_route.next()?;

        let consumers_info = self.flow_controls.get_consumers_info(&self.flow_control_id);

        if consumers_info.contains(next) {
            return crate::allow();
        }

        if let Some(spawner_flow_control_id) = &self.spawner_flow_control_id {
            let consumers_info = self
                .flow_controls
                .get_consumers_info(spawner_flow_control_id);

            if consumers_info.contains(next) {
                return crate::allow();
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
