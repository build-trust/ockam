use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for a Forwarding Service
pub struct ForwardingServiceOptions {
    pub(super) service_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) forwarders_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) consumer_service: Vec<FlowControlId>,
    pub(super) consumer_forwarder: Vec<FlowControlId>,
}

impl ForwardingServiceOptions {
    /// Default constructor without Access Control
    pub fn new() -> Self {
        Self {
            service_incoming_access_control: Arc::new(AllowAll),
            forwarders_incoming_access_control: Arc::new(AllowAll),
            consumer_service: vec![],
            consumer_forwarder: vec![],
        }
    }

    /// Mark that this Forwarding service is a Consumer for to the given [`FlowControlId`]
    pub fn service_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_service.push(id.clone());

        self
    }

    /// Mark that spawned Forwarders are Consumers for to the given [`FlowControlId`]
    pub fn forwarder_as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer_forwarder.push(id.clone());

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

    /// Set spawned forwarders Incoming Access Control
    pub fn with_forwarders_incoming_access_control_impl(
        mut self,
        access_control: impl IncomingAccessControl,
    ) -> Self {
        self.forwarders_incoming_access_control = Arc::new(access_control);
        self
    }

    /// Set spawned forwarders Incoming Access Control
    pub fn with_forwarders_incoming_access_control(
        mut self,
        access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.forwarders_incoming_access_control = access_control;
        self
    }

    pub(super) fn setup_flow_control_for_forwarding_service(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_service {
            flow_controls.add_consumer(address.clone(), id);
        }
    }

    pub(super) fn setup_flow_control_for_forwarder(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_forwarder {
            flow_controls.add_consumer(address.clone(), id);
        }
    }
}

impl Default for ForwardingServiceOptions {
    fn default() -> Self {
        Self::new()
    }
}
