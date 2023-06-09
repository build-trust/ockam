use crate::portal::addresses::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{
    FlowControls, ProducerFlowControlId, SpawnerFlowControlId, SpawnerFlowControlPolicy,
};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for an Inlet
#[derive(Debug)]
pub struct TcpInletOptions {
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpInletOptions {
    /// Default constructor without Incoming Access Control
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
            flow_controls.add_consumer_for_producer(addresses.remote.clone(), &flow_control_id);
        }
    }
}

impl Default for TcpInletOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub(super) struct SpawnerConsumer {
    pub(super) id: SpawnerFlowControlId,
    pub(super) policy: SpawnerFlowControlPolicy,
}

/// Trust Options for an Outlet
#[derive(Debug)]
pub struct TcpOutletOptions {
    pub(super) consumer_for_spawner_flow_control: Vec<SpawnerConsumer>,
    pub(super) consumer_for_producer_flow_control: Vec<ProducerFlowControlId>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpOutletOptions {
    /// Default constructor without Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_for_spawner_flow_control: vec![],
            consumer_for_producer_flow_control: vec![],
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

    /// Mark that this Outlet listener is a Consumer for to the given [`SpawnerFlowControlId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`ProducerFlowControlId`]
    /// of the message that was used to create the Outlet
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

    /// Mark that this Outlet listener is a Consumer for to the given [`ProducerFlowControlId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`ProducerFlowControlId`]
    /// of the message that was used to create the Outlet
    pub fn as_consumer_for_producer(mut self, id: &ProducerFlowControlId) -> Self {
        self.consumer_for_producer_flow_control.push(id.clone());

        self
    }

    pub(super) fn setup_flow_control_for_outlet_listener(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for id in &self.consumer_for_producer_flow_control {
            flow_controls.add_consumer_for_producer(address.clone(), id);
        }
        for spawner_consumer in &self.consumer_for_spawner_flow_control {
            flow_controls.add_consumer_for_spawner(
                address.clone(),
                &spawner_consumer.id,
                spawner_consumer.policy,
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
            flow_controls
                .add_consumer_for_producer(addresses.remote.clone(), &producer_flow_control_id);
        }
    }
}

impl Default for TcpOutletOptions {
    fn default() -> Self {
        Self::new()
    }
}
