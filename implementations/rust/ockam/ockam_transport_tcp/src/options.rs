use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlId, FlowControlOutgoingAccessControl, FlowControlPolicy, FlowControls,
};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};

pub(crate) struct TcpConnectionAccessControl {
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

#[derive(Debug)]
pub(super) struct ConsumerFlowControl {
    pub(super) flow_control_id: FlowControlId,
    pub(super) flow_control_policy: FlowControlPolicy,
}

/// Trust Options for a TCP connection
#[derive(Debug)]
pub struct TcpConnectionOptions {
    pub(super) consumer_flow_control: Vec<ConsumerFlowControl>,
    pub(crate) producer_flow_control_id: FlowControlId,
}

impl TcpConnectionOptions {
    #[allow(clippy::new_without_default)]
    /// Mark this Tcp Receiver as a Producer with a random [`FlowControlId`]
    pub fn new() -> Self {
        Self {
            consumer_flow_control: vec![],
            producer_flow_control_id: FlowControls::generate_id(),
        }
    }

    /// Mark that this Connection is a Consumer for to the given [`FlowControlId`]
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

    /// Getter for freshly generated [`FlowControlId`]
    pub fn producer_flow_control_id(&self) -> FlowControlId {
        self.producer_flow_control_id.clone()
    }
}

impl TcpConnectionOptions {
    pub(crate) fn setup_flow_control(&self, flow_controls: &FlowControls, addresses: &Addresses) {
        flow_controls.add_producer(
            addresses.receiver_address().clone(),
            &self.producer_flow_control_id,
            None,
            vec![addresses.sender_address().clone()],
        );
        for consumer_flow_control in &self.consumer_flow_control {
            flow_controls.add_consumer(
                addresses.sender_address().clone(),
                &consumer_flow_control.flow_control_id,
                consumer_flow_control.flow_control_policy,
            );
        }
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
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            spawner_flow_control_id: FlowControls::generate_id(),
        }
    }

    /// Getter for freshly generated [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> FlowControlId {
        self.spawner_flow_control_id.clone()
    }
}

impl TcpListenerOptions {
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
