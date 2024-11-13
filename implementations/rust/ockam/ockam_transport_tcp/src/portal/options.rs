use crate::portal::addresses::Addresses;
use crate::TlsCertificateProvider;
use ockam_core::compat::sync::Arc;
use ockam_core::env::get_env_with_default_ignore_error;
use ockam_core::flow_control::{FlowControlId, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};

/// Maximum allowed size for a payload for TCP Portal
pub fn read_portal_payload_length() -> usize {
    get_env_with_default_ignore_error("OCKAM_TCP_PORTAL_PAYLOAD_LENGTH", 128 * 1024)
}

/// Options for an Inlet
#[derive(Clone, Debug)]
pub struct TcpInletOptions {
    pub(crate) incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(crate) outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub(crate) is_paused: bool,
    pub(crate) tls_certificate_provider: Option<Arc<dyn TlsCertificateProvider>>,
    pub(crate) portal_payload_length: usize,
}

impl TcpInletOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            incoming_access_control: Arc::new(AllowAll),
            outgoing_access_control: Arc::new(AllowAll),
            is_paused: false,
            tls_certificate_provider: None,
            portal_payload_length: read_portal_payload_length(),
        }
    }

    /// Set TCP inlet to paused mode after start. No unpause call [`TcpInlet::unpause`]
    pub fn paused(mut self) -> Self {
        self.is_paused = true;
        self
    }

    /// Set TLS certificate provider.
    /// Whe omitted the inlet will be clear-text
    pub fn with_tls_certificate_provider(
        mut self,
        tls_certificate: Arc<dyn TlsCertificateProvider>,
    ) -> Self {
        self.tls_certificate_provider = Some(tls_certificate);
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

    pub(crate) fn setup_flow_control(
        flow_controls: &FlowControls,
        addresses: &Addresses,
        next: &Address,
    ) {
        Self::setup_flow_control_for_address(flow_controls, &addresses.sender_remote, next)
    }

    pub(crate) fn setup_flow_control_for_address(
        flow_controls: &FlowControls,
        address: &Address,
        next: &Address,
    ) {
        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(next)
            .map(|x| x.flow_control_id().clone())
        {
            // Allow a sender with corresponding flow_control_id send messages to this address
            flow_controls.add_consumer(address, &flow_control_id);
        }
    }
}

impl Default for TcpInletOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for an Outlet
#[derive(Clone, Debug)]
pub struct TcpOutletOptions {
    pub(crate) consumer: Vec<FlowControlId>,
    pub(crate) incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(crate) outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub(crate) tls: bool,
    pub(crate) portal_payload_length: usize,
}

impl TcpOutletOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer: vec![],
            incoming_access_control: Arc::new(AllowAll),
            outgoing_access_control: Arc::new(AllowAll),
            tls: false,
            portal_payload_length: read_portal_payload_length(),
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

    pub(crate) fn setup_flow_control_for_outlet_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer {
            flow_controls.add_consumer(address, id);
        }
    }

    pub(crate) fn setup_flow_control_for_outlet(
        flow_controls: &FlowControls,
        addresses: &Addresses,
        src_addr: &Address,
    ) {
        // Check if the Worker that send us this message is a Producer
        // If yes - outlet worker will be added to that flow control to be able to receive further
        // messages from that Producer
        if let Some(producer_info) = flow_controls.get_flow_control_with_producer(src_addr) {
            flow_controls.add_consumer(&addresses.sender_remote, producer_info.flow_control_id());
        }
    }
}

impl Default for TcpOutletOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(non_snake_case)]
#[test]
fn tcp_portal_options_portal_length__env_var_set__pulls_correct_value() {
    let length: usize = rand::random();
    std::env::set_var("OCKAM_TCP_PORTAL_PAYLOAD_LENGTH", length.to_string());

    assert_eq!(TcpInletOptions::default().portal_payload_length, length);
    assert_eq!(TcpOutletOptions::default().portal_payload_length, length);
}
