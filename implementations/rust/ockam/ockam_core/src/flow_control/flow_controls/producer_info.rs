use crate::flow_control::FlowControlId;

/// Producer information
#[derive(Clone, Debug)]
pub struct ProducerInfo {
    pub(super) flow_control_id: FlowControlId,
    pub(super) spawner_flow_control_id: Option<FlowControlId>,
}

impl ProducerInfo {
    /// [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }

    /// Spawner's [`FlowControlId`]
    pub fn spawner_flow_control_id(&self) -> &Option<FlowControlId> {
        &self.spawner_flow_control_id
    }
}
