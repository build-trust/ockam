use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::flow_control::{
    FlowControls, ProducerFlowControlId, SpawnerFlowControlId, SpawnerFlowControlPolicy,
};
use ockam_core::{Address, AllowAll, IncomingAccessControl};

/// Trust Options for a Forwarding Service
pub struct ForwardingServiceOptions {
    pub(super) service_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) forwarders_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub(super) consumer_for_spawner_service_flow_control: Vec<SpawnerConsumer>,
    pub(super) consumer_for_producer_service_flow_control: Vec<ProducerFlowControlId>,
    pub(super) consumer_for_spawner_forwarder_flow_control: Vec<SpawnerConsumer>,
    pub(super) consumer_for_producer_forwarder_flow_control: Vec<ProducerFlowControlId>,
}

pub(super) struct SpawnerConsumer {
    pub(super) id: SpawnerFlowControlId,
    pub(super) policy: SpawnerFlowControlPolicy,
}

impl ForwardingServiceOptions {
    /// Default constructor without Access Control
    pub fn new() -> Self {
        Self {
            service_incoming_access_control: Arc::new(AllowAll),
            forwarders_incoming_access_control: Arc::new(AllowAll),
            consumer_for_spawner_service_flow_control: vec![],
            consumer_for_producer_service_flow_control: vec![],
            consumer_for_spawner_forwarder_flow_control: vec![],
            consumer_for_producer_forwarder_flow_control: vec![],
        }
    }

    /// Mark that this Forwarding service is a Consumer for to the given [`ProducerFlowControlId`]
    pub fn service_as_consumer_for_producer(mut self, id: &ProducerFlowControlId) -> Self {
        self.consumer_for_producer_service_flow_control
            .push(id.clone());

        self
    }

    /// Mark that this Forwarding service is a Consumer for to the given [`SpawnerFlowControlId`]
    pub fn service_as_consumer_for_spawner(
        mut self,
        id: &SpawnerFlowControlId,
        policy: SpawnerFlowControlPolicy,
    ) -> Self {
        self.consumer_for_spawner_service_flow_control
            .push(SpawnerConsumer {
                id: id.clone(),
                policy,
            });

        self
    }

    /// Mark that spawned Forwarders are Consumers for to the given [`ProducerFlowControlId`]
    pub fn forwarder_as_consumer_for_producer(mut self, id: &ProducerFlowControlId) -> Self {
        self.consumer_for_producer_forwarder_flow_control
            .push(id.clone());

        self
    }

    /// Mark that spawned Forwarders are Consumers for to the given [`SpawnerFlowControlId`]
    pub fn forwarder_as_consumer_for_spawner(
        mut self,
        id: &SpawnerFlowControlId,
        policy: SpawnerFlowControlPolicy,
    ) -> Self {
        self.consumer_for_spawner_forwarder_flow_control
            .push(SpawnerConsumer {
                id: id.clone(),
                policy,
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
        for spawner_consumer in &self.consumer_for_spawner_service_flow_control {
            flow_controls.add_consumer_for_spawner(
                address.clone(),
                &spawner_consumer.id,
                spawner_consumer.policy,
            );
        }
        for id in &self.consumer_for_producer_service_flow_control {
            flow_controls.add_consumer_for_producer(address.clone(), id);
        }
    }

    pub(super) fn setup_flow_control_for_forwarder(
        &self,
        flow_controls: &FlowControls,
        address: &Address,
    ) {
        for spawner_consumer in &self.consumer_for_spawner_forwarder_flow_control {
            flow_controls.add_consumer_for_spawner(
                address.clone(),
                &spawner_consumer.id,
                spawner_consumer.policy,
            );
        }
        for id in &self.consumer_for_producer_forwarder_flow_control {
            flow_controls.add_consumer_for_producer(address.clone(), id);
        }
    }
}

impl Default for ForwardingServiceOptions {
    fn default() -> Self {
        Self::new()
    }
}
