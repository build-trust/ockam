use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for a Forwarding Service
pub struct RelayServiceOptions {
    pub(super) service_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) relays_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) consumer_service: Vec<FlowControlId>,
    pub(super) consumer_relay: Vec<FlowControlId>,
}

impl RelayServiceOptions {
    /// Default constructor without Access Control
    pub fn new() -> Self {
        Self {
            service_incoming_access_control: Arc::new(AllowAll),
            relays_incoming_access_control: Arc::new(AllowAll),
            consumer_service: vec![],
            consumer_relay: vec![],
        }
    }

    /// Mark that this Relay service is a Consumer for to the given [`FlowControlId`]
    pub fn service_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_service.push(id.clone());

        self
    }

    /// Mark that spawned Relays are Consumers for to the given [`FlowControlId`]
    pub fn relay_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_relay.push(id.clone());

        self
    }

    /// Set Service Incoming Access Control
    pub fn with_service_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.service_incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set Service Incoming Access Control
    pub fn with_service_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.service_incoming_access_control = access_control;
        self
    }

    /// Set spawned relays Incoming Access Control
    pub fn with_relays_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.relays_incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set spawned relays Incoming Access Control
    pub fn with_relays_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.relays_incoming_access_control = access_control;
        self
    }

    pub(super) fn setup_flow_control_for_relay_service(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_service {
            flow_controls.add_consumer(address.clone(), id);
        }
    }

    pub(super) fn setup_flow_control_for_relay(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_relay {
            flow_controls.add_consumer(address.clone(), id);
        }
    }
}

impl Default for RelayServiceOptions {
    fn default() -> Self {
        Self::new()
    }
}
