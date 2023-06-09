use crate::flow_control::{ProducerFlowControlId, SpawnerFlowControlId};

/// Producer information
#[derive(Clone, Debug)]
pub struct ProducerInfo {
    pub(super) flow_control_id: ProducerFlowControlId,
    pub(super) spawner_flow_control_id: Option<SpawnerFlowControlId>,
}

impl ProducerInfo {
    /// [`FlowControlId`]
    pub fn flow_control_id(&self) -> &ProducerFlowControlId {
        &self.flow_control_id
    }

    /// Spawner's [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> &Option<SpawnerFlowControlId> {
        &self.spawner_flow_control_id
    }
}
