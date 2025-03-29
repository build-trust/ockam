use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};
use std::sync::Arc;

/// Trust Options for a `UdpPunctureNegotiationListener`
#[derive(Debug)]
pub struct UdpPunctureNegotiationListenerOptions {
    pub(super) flow_control_id: FlowControlId,
    pub(super) consumer: Vec<FlowControlId>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl UdpPunctureNegotiationListenerOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            flow_control_id: FlowControls::generate_flow_control_id(),
            consumer: vec![],
            incoming_access_control: Arc::new(AllowAll),
            outgoing_access_control: Arc::new(AllowAll),
        }
    }

    /// Set Incoming Access Control
    pub fn with_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set Incoming Access Control
    pub fn with_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.incoming_access_control = access_control;
        self
    }

    /// Set Outgoing Access Control
    pub fn with_outgoing_access_control_impl(
        mut self,
        access_control: impl OutgoingAccessControl,
    ) -> Self {
        self.outgoing_access_control = Arc::new(access_control);
        self
    }

    /// Set Outgoing Access Control
    pub fn with_outgoing_access_control(
        mut self,
        access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        self.outgoing_access_control = access_control;
        self
    }

    /// Mark that this UDP Negotiation listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned workers will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the `NegotiationWorker`
    #[allow(clippy::wrong_self_convention)]
    pub fn as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer.push(id.clone());

        self
    }

    pub(super) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer {
            flow_controls.add_consumer(address, id);
        }

        flow_controls.add_spawner(address, &self.flow_control_id);
    }

    /// Spawner [`FlowControlId`]
    pub fn flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }
}

impl Default for UdpPunctureNegotiationListenerOptions {
    fn default() -> Self {
        Self::new()
    }
}
