use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControlOutgoingAccessControl, FlowControls, ProducerFlowControlId, SpawnerFlowControlId,
    SpawnerFlowControlPolicy,
};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl};

pub(crate) struct TcpConnectionAccessControl {
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

#[derive(Debug)]
pub(super) struct SpawnerConsumer {
    pub(super) id: SpawnerFlowControlId,
    pub(super) policy: SpawnerFlowControlPolicy,
}

/// Trust Options for a TCP connection
#[derive(Debug)]
pub struct TcpConnectionOptions {
    pub(super) consumer_for_spawner_flow_control: Vec<SpawnerConsumer>,
    pub(super) consumer_for_producer_flow_control: Vec<ProducerFlowControlId>,
    pub(crate) flow_control_id: ProducerFlowControlId,
}

impl TcpConnectionOptions {
    #[allow(clippy::new_without_default)]
    /// Mark this Tcp Receiver as a Producer with a random [`FlowControlId`]
    pub fn new() -> Self {
        Self {
            consumer_for_spawner_flow_control: vec![],
            consumer_for_producer_flow_control: vec![],
            flow_control_id: FlowControls::generate_producer_flow_control_id(),
        }
    }

    /// Mark that this Connection is a Consumer for to the given [`FlowControlId`]
    pub fn as_consumer_for_spawner(
        mut self,
        id: &SpawnerFlowControlId,
        policy: SpawnerFlowControlPolicy,
    ) -> Self {
        self.consumer_for_spawner_flow_control
            .push(SpawnerConsumer {
                id: id.clone(),
                policy,
            });

        self
    }

    /// Mark that this Connection is a Consumer for to the given [`FlowControlId`]
    pub fn as_consumer_for_producer(mut self, id: &ProducerFlowControlId) -> Self {
        self.consumer_for_producer_flow_control.push(id.clone());

        self
    }

    /// Getter for freshly generated [`FlowControlId`]
    pub fn flow_control_id(&self) -> ProducerFlowControlId {
        self.flow_control_id.clone()
    }
}

impl TcpConnectionOptions {
    pub(crate) fn setup_flow_control(&self, flow_controls: &FlowControls, addresses: &Addresses) {
        flow_controls.add_producer(
            addresses.receiver_address().clone(),
            &self.flow_control_id,
            None,
            vec![addresses.sender_address().clone()],
        );
        for spawner_consumer in &self.consumer_for_spawner_flow_control {
            flow_controls.add_consumer_for_spawner(
                addresses.sender_address().clone(),
                &spawner_consumer.id,
                spawner_consumer.policy,
            );
        }

        for id in &self.consumer_for_producer_flow_control {
            flow_controls.add_consumer_for_producer(addresses.sender_address().clone(), id);
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
                self.flow_control_id,
                None,
            )),
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Debug)]
pub struct TcpListenerOptions {
    pub(crate) flow_control_id: SpawnerFlowControlId,
}

impl TcpListenerOptions {
    /// Mark this Tcp Listener as a Spawner with given [`FlowControlId`].
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            flow_control_id: FlowControls::generate_spawner_flow_control_id(),
        }
    }

    /// Getter for freshly generated [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> SpawnerFlowControlId {
        self.flow_control_id.clone()
    }
}

impl TcpListenerOptions {
    pub(crate) fn setup_flow_control_for_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        flow_controls.add_spawner(address.clone(), &self.flow_control_id);
    }

    pub(crate) fn setup_flow_control_for_connection(
        &self,
        flow_controls: &FlowControls,
        addresses: &Addresses,
    ) -> ProducerFlowControlId {
        let flow_control_id = FlowControls::generate_producer_flow_control_id();

        flow_controls.add_producer(
            addresses.receiver_address().clone(),
            &flow_control_id,
            Some(&self.flow_control_id),
            vec![addresses.sender_address().clone()],
        );

        flow_control_id
    }

    pub(crate) fn create_access_control(
        &self,
        flow_controls: &FlowControls,
        flow_control_id: ProducerFlowControlId,
    ) -> TcpConnectionAccessControl {
        TcpConnectionAccessControl {
            sender_incoming_access_control: Arc::new(AllowAll),
            receiver_outgoing_access_control: Arc::new(FlowControlOutgoingAccessControl::new(
                flow_controls,
                flow_control_id,
                Some(self.flow_control_id.clone()),
            )),
        }
    }
}
