use crate::portal::addresses::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for an Inlet
pub struct TcpInletOptions {
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpInletOptions {
    /// Default constructor without flow control and Incoming Access Control
    pub fn new() -> Self {
        Self {
            incoming_access_control: Arc::new(AllowAll),
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

    pub(super) fn setup_flow_control(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(
                addresses.remote.clone(),
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }
    }
}

impl Default for TcpInletOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) struct ConsumerFlowControl {
    pub(super) flow_control_id: FlowControlId,
    pub(super) flow_control_policy: FlowControlPolicy,
}

/// Trust Options for an Outlet
pub struct TcpOutletOptions {
    pub(super) consumer_flow_control: Vec<ConsumerFlowControl>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpOutletOptions {
    /// Default constructor without flow control and Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_flow_control: vec![],
            incoming_access_control: Arc::new(AllowAll),
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

    /// Mark that this Outlet listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Outlet
    pub fn as_consumer(
        mut self,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_flow_control.push(ConsumerFlowControl {
            flow_control_id: flow_control_id.clone(),
            flow_control_policy,
        });

        self
    }

    pub(super) fn setup_flow_control_for_outlet_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for consumer_flow_control in &self.consumer_flow_control {
            flow_controls.add_consumer(
                address.clone(),
                &consumer_flow_control.flow_control_id,
                consumer_flow_control.flow_control_policy,
            );
        }
    }

    pub(super) fn setup_flow_control_for_outlet(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
        src_addr: &Address,
    ) {
        // Check if the Worker that send us this message is a Producer
        // If yes - outlet worker will be added to that flow control to be able to receive further
        // messages from that Producer
        if let Some(producer_flow_control_id) = flow_controls
            .get_flow_control_with_producer(src_addr)
            .map(|x| x.flow_control_id().clone())
        {
            flow_controls.add_consumer(
                addresses.remote.clone(),
                &producer_flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }
    }
}

impl Default for TcpOutletOptions {
    fn default() -> Self {
        Self::new()
    }
}
