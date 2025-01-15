use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{Address, OutgoingAccessControl};

/// Trust Options for a TCP connection
#[derive(Debug)]
pub struct TcpConnectionOptions {
    pub(super) timeout: Option<Duration>,
    pub(super) consumer: Vec<FlowControlId>,
    pub(crate) flow_control_id: FlowControlId,
}

impl TcpConnectionOptions {
    #[allow(clippy::new_without_default)]
    /// Mark this Tcp Receiver as a Producer with a random [`FlowControlId`]
    pub fn new() -> Self {
        Self {
            timeout: None,
            consumer: vec![],
            flow_control_id: FlowControls::generate_flow_control_id(),
        }
    }

    /// Mark that this Connection is a Consumer for to the given [`FlowControlId`]
    pub fn as_consumer(mut self, id: &FlowControlId) -> Self {
        self.consumer.push(id.clone());

        self
    }

    /// Getter for freshly generated [`FlowControlId`]
    pub fn flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }

    /// Set connect timeout
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set connect timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Connect timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

impl TcpConnectionOptions {
    pub(crate) fn setup_flow_control(&self, flow_controls: &FlowControls, addresses: &Addresses) {
        flow_controls.add_producer(
            addresses.receiver_address(),
            &self.flow_control_id,
            None,
            vec![addresses.sender_address().clone()],
        );

        for id in &self.consumer {
            flow_controls.add_consumer(addresses.sender_address(), id);
        }
    }

    pub(crate) fn create_receiver_outgoing_access_control(
        self,
        flow_controls: &FlowControls,
    ) -> Arc<dyn OutgoingAccessControl> {
        Arc::new(FlowControlOutgoingAccessControl::new(
            flow_controls,
            self.flow_control_id,
            None,
        ))
    }
}

/// Trust Options for a TCP listener
#[derive(Debug)]
pub struct TcpListenerOptions {
    pub(crate) flow_control_id: FlowControlId,
}

impl TcpListenerOptions {
    /// Mark this Tcp Listener as a Spawner with given [`FlowControlId`].
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            flow_control_id: FlowControls::generate_flow_control_id(),
        }
    }

    /// Getter for freshly generated [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> FlowControlId {
        self.flow_control_id.clone()
    }
}

impl TcpListenerOptions {
    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        flow_controls.add_spawner(address, &self.flow_control_id);
    }

    pub(crate) fn setup_flow_control_for_connection(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
    ) -> FlowControlId {
        let flow_control_id = FlowControls::generate_flow_control_id();

        flow_controls.add_producer(
            addresses.receiver_address(),
            &flow_control_id,
            Some(&self.flow_control_id),
            vec![addresses.sender_address().clone()],
        );

        flow_control_id
    }

    pub(crate) fn create_receiver_outgoing_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: FlowControlId,
    ) -> Arc<dyn OutgoingAccessControl> {
        Arc::new(FlowControlOutgoingAccessControl::new(
            flow_controls,
            flow_control_id,
            Some(self.flow_control_id.clone()),
        ))
    }
}
