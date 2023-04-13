use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};

pub(crate) struct TcpConnectionAccessControl {
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

/// Trust Options for a TCP connection
#[derive(Clone, Debug)]
pub struct TcpConnectionOptions {
    pub(crate) producer_flow_control_id: FlowControlId,
}

impl TcpConnectionOptions {
    #[allow(clippy::new_without_default)]
    /// Mark this Tcp Receiver as a Producer with a random [`FlowControlId`]
    pub fn new() -> Self {
        Self {
            producer_flow_control_id: FlowControls::generate_id(),
        }
    }

    /// Mark this Tcp Receiver as a Producer for a given [`FlowControlId`]
    pub fn as_producer(flow_control_id: &FlowControlId) -> Self {
        Self {
            producer_flow_control_id: flow_control_id.clone(),
        }
    }

    pub(crate) fn setup_flow_control(&self, flow_controls: &FlowControls, addresses: &Addresses) {
        flow_controls.add_producer(
            addresses.receiver_address().clone(),
            &self.producer_flow_control_id,
            None,
            vec![addresses.sender_address().clone()],
        );
    }

    pub(crate) fn create_access_control(
        self,
        flow_controls: &FlowControls,
    ) -> TcpConnectionAccessControl {
        TcpConnectionAccessControl {
            sender_incoming_access_control: Arc::new(AllowAll),
            receiver_outgoing_access_control: Arc::new(FlowControlOutgoingAccessControl::new(
                flow_controls,
                self.producer_flow_control_id,
                None,
            )),
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Debug)]
pub struct TcpListenerOptions {
    pub(crate) spawner_flow_control_id: FlowControlId,
}

impl TcpListenerOptions {
    /// Mark this Tcp Listener as a Spawner with given [`FlowControlId`].
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    pub fn new(flow_control_id: &FlowControlId) -> Self {
        Self {
            spawner_flow_control_id: flow_control_id.clone(),
        }
    }

    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        flow_controls.add_spawner(address.clone(), &self.spawner_flow_control_id);
    }

    pub(crate) fn setup_flow_control_for_connection(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
    ) -> FlowControlId {
        let flow_control_id = FlowControls::generate_id();

        flow_controls.add_producer(
            addresses.receiver_address().clone(),
            &flow_control_id,
            Some(&self.spawner_flow_control_id),
            vec![addresses.sender_address().clone()],
        );

        flow_control_id
    }

    pub(crate) fn create_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: FlowControlId,
    ) -> TcpConnectionAccessControl {
        TcpConnectionAccessControl {
            sender_incoming_access_control: Arc::new(AllowAll),
            receiver_outgoing_access_control: Arc::new(FlowControlOutgoingAccessControl::new(
                flow_controls,
                flow_control_id,
                Some(self.spawner_flow_control_id.clone()),
            )),
        }
    }
}
