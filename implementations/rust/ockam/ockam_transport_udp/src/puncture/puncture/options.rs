use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{Address, OutgoingAccessControl, Result};

use crate::puncture::puncture::Addresses;
use core::fmt;
use core::fmt::Formatter;

/// Options for a UDP puncture
pub struct UdpPunctureOptions {
    pub(crate) flow_control_id: FlowControlId,
    pub(crate) spawner_flow_control_id: Option<FlowControlId>,
}

impl fmt::Debug for UdpPunctureOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "FlowId: {}", self.flow_control_id)
    }
}

impl UdpPunctureOptions {
    /// Mark this UDP puncture as a Producer with a random [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            flow_control_id: FlowControls::generate_flow_control_id(),
            spawner_flow_control_id: None,
        }
    }

    /// Mark this UDP puncture as a Producer with a random [`FlowControlId`] and given spawner's
    /// [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new_with_spawner(spawner_flow_control_id: FlowControlId) -> Self {
        Self {
            flow_control_id: FlowControls::generate_flow_control_id(),
            spawner_flow_control_id: Some(spawner_flow_control_id),
        }
    }

    /// Freshly generated [`FlowControlId`]
    pub fn producer_flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }
}

impl UdpPunctureOptions {
    pub(crate) fn setup_flow_control(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) -> Result<()> {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(addresses.remote_address(), &flow_control_id);
        }

        flow_controls.add_producer(
            addresses.receiver_address(),
            &self.flow_control_id,
            None,
            vec![addresses.sender_address().clone()],
        );

        Ok(())
    }

    pub(crate) fn create_receiver_outgoing_access_control(
        &self,
        flow_controls: &FlowControls,
    ) -> Arc<dyn OutgoingAccessControl> {
        let ac = FlowControlOutgoingAccessControl::new(
            flow_controls,
            self.flow_control_id.clone(),
            self.spawner_flow_control_id.clone(),
        );

        Arc::new(ac)
    }
}
