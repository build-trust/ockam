use crate::portal::addresses::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};

/// Trust Options for an Inlet
#[derive(Debug)]
pub struct TcpInletOptions {
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub(super) is_paused: bool,
}

impl TcpInletOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            incoming_access_control: Arc::new(AllowAll),
            outgoing_access_control: Arc::new(AllowAll),
            is_paused: false,
        }
    }

    /// Set TCP inlet to paused mode after start. No unpause call [`TcpInlet::unpause`]
    pub fn paused(mut self) -> Self {
        self.is_paused = true;
        self
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
            flow_controls.add_consumer(addresses.sender_remote.clone(), &flow_control_id);
        }
    }
}

impl Default for TcpInletOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Trust Options for an Outlet
#[derive(Debug)]
pub struct TcpOutletOptions {
    pub(super) consumer: Vec<FlowControlId>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub(super) tls: bool,
}

impl TcpOutletOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer: vec![],
            incoming_access_control: Arc::new(AllowAll),
            outgoing_access_control: Arc::new(AllowAll),
            tls: false,
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

    /// Set TLS
    pub fn with_tls(mut self, tls: bool) -> Self {
        self.tls = tls;
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

    /// Mark that this Outlet listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Outlet
    pub fn as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer.push(id.clone());

        self
    }

    pub(super) fn setup_flow_control_for_outlet_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer {
            flow_controls.add_consumer(address.clone(), id);
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
            flow_controls.add_consumer(addresses.sender_remote.clone(), &producer_flow_control_id);
        }
    }
}

impl Default for TcpOutletOptions {
    fn default() -> Self {
        Self::new()
    }
}
