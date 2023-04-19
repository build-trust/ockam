use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for a Forwarding Service
pub struct ForwardingServiceOptions {
    pub(super) service_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) forwarders_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) consumer_service_flow_control: Option<ConsumerFlowControl>,
    pub(super) consumer_forwarder_flow_control: Option<ConsumerFlowControl>,
}

pub(super) struct ConsumerFlowControl {
    pub(super) flow_control_id: FlowControlId,
    pub(super) flow_control_policy: FlowControlPolicy,
}

impl ForwardingServiceOptions {
    /// Default constructor without Access Control
    pub fn new() -> Self {
        Self {
            service_incoming_access_control: Arc::new(AllowAll),
            forwarders_incoming_access_control: Arc::new(AllowAll),
            consumer_service_flow_control: None,
            consumer_forwarder_flow_control: None,
        }
    }

    /// Mark that this Forwarding service is a Consumer for to the given [`FlowControlId`]
    pub fn service_as_consumer(
        mut self,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_service_flow_control = Some(ConsumerFlowControl {
            flow_control_id: flow_control_id.clone(),
            flow_control_policy,
        });

        self
    }

    /// Mark that spawned Forwarders are Consumers for to the given [`FlowControlId`]
    pub fn forwarder_as_consumer(
        mut self,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_forwarder_flow_control = Some(ConsumerFlowControl {
            flow_control_id: flow_control_id.clone(),
            flow_control_policy,
        });

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
        if let Some(consumer_flow_control) = &self.consumer_service_flow_control {
            flow_controls.add_consumer(
                address.clone(),
                &consumer_flow_control.flow_control_id,
                consumer_flow_control.flow_control_policy,
            );
        }
    }

    pub(super) fn setup_flow_control_for_forwarder(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        if let Some(consumer_flow_control) = &self.consumer_forwarder_flow_control {
            flow_controls.add_consumer(
                address.clone(),
                &consumer_flow_control.flow_control_id,
                consumer_flow_control.flow_control_policy,
            );
        }
    }
}

impl Default for ForwardingServiceOptions {
    fn default() -> Self {
        Self::new()
    }
}
