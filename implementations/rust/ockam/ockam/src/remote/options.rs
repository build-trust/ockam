use crate::remote::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, AllowAll, OutgoingAccessControl};

/// Trust options for [`RemoteForwarder`]
pub struct RemoteForwarderOptions {
    pub(super) flow_controls: Option<FlowControls>,
}

impl RemoteForwarderOptions {
    /// This constructor is insecure, because outgoing messages from such forwarder will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            flow_controls: None,
        }
    }

    /// Mark this [`RemoteForwarder`] as a Producer and Consumer for a given [`FlowControlId`]
    /// Usually [`FlowControlId`] should be shared with the Producer that was used to create this
    /// forwarder (probably Secure Channel), since [`RemoteForwarder`] doesn't imply any new "trust"
    /// context, it's just a Message Routing helper. Therefore, workers that are allowed to receive
    /// messages from the corresponding Secure Channel should as well be allowed to receive messages
    /// through the [`RemoteForwarder`] through the same Secure Channel.
    pub fn as_consumer_and_producer(flow_controls: &FlowControls) -> Self {
        Self {
            flow_controls: Some(flow_controls.clone()),
        }
    }

    pub(super) fn setup_flow_control(
        &self,
        addresses: &Addresses,
        next: &Address,
    ) -> Option<FlowControlId> {
        match &self.flow_controls {
            Some(flow_controls) => {
                match flow_controls
                    .find_flow_control_with_producer_address(next)
                    .map(|x| x.flow_control_id().clone())
                {
                    Some(flow_control_id) => {
                        // Allow a sender with corresponding flow_control_id send messages to this address
                        flow_controls.add_consumer(
                            &addresses.main_remote,
                            &flow_control_id,
                            FlowControlPolicy::ProducerAllowMultiple,
                        );

                        flow_controls.add_producer(
                            &addresses.main_internal,
                            &flow_control_id,
                            None,
                            vec![],
                        );

                        Some(flow_control_id)
                    }
                    None => None,
                }
            }
            None => None,
        }
    }

    pub(super) fn create_access_control(
        &self,
        flow_control_id: Option<FlowControlId>,
    ) -> Arc<dyn OutgoingAccessControl> {
        if let (Some(flow_controls), Some(flow_control_id)) = (&self.flow_controls, flow_control_id)
        {
            let ac =
                FlowControlOutgoingAccessControl::new(flow_controls.clone(), flow_control_id, None);

            Arc::new(ac)
        } else {
            Arc::new(AllowAll)
        }
    }
}
