use crate::remote::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, AllowAll, OutgoingAccessControl};

/// Trust options for [`RemoteForwarder`](super::RemoteForwarder)
pub struct RemoteForwarderOptions {}

impl RemoteForwarderOptions {
    /// Usually [`FlowControlId`] should be shared with the Producer that was used to create this
    /// forwarder (probably Secure Channel), since [`RemoteForwarder`](super::RemoteForwarder)
    /// doesn't imply any new "trust" context, it's just a Message Routing helper.
    /// Therefore, workers that are allowed to receive messages from the corresponding
    /// Secure Channel should as well be allowed to receive messages
    /// through the [`RemoteForwarder`](super::RemoteForwarder) through the same Secure Channel.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {}
    }

    pub(super) fn setup_flow_control(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) -> Option<FlowControlId> {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(
                addresses.main_remote.clone(),
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );

            flow_controls.add_producer(
                addresses.main_internal.clone(),
                &flow_control_id,
                None,
                vec![],
            );

            Some(flow_control_id)
        } else {
            None
        }
    }

    pub(super) fn create_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: Option<FlowControlId>,
    ) -> Arc<dyn OutgoingAccessControl> {
        if let Some(flow_control_id) = flow_control_id {
            let ac = FlowControlOutgoingAccessControl::new(flow_controls, flow_control_id, None);

            Arc::new(ac)
        } else {
            Arc::new(AllowAll)
        }
    }
}
