use crate::portal::addresses::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlPolicy, FlowControls};
use ockam_core::{Address, AllowAll, IncomingAccessControl, Result};
use ockam_transport_core::TransportError;

/// Trust Options for an Inlet
pub struct TcpInletTrustOptions {
    pub(super) consumer_flow_controls: Option<FlowControls>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpInletTrustOptions {
    /// Default constructor without flow control and Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_flow_controls: None,
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

    /// Mark that created Inlets are Consumer for to the given [`FlowControlId`]
    pub fn as_consumer(mut self, flow_controls: &FlowControls) -> Self {
        self.consumer_flow_controls = Some(flow_controls.clone());

        self
    }

    pub(super) fn setup_flow_control(&self, addresses: &Addresses, next: &Address) -> Result<()> {
        match &self.consumer_flow_controls {
            Some(flow_controls) => {
                if let Some(flow_control_id) = flow_controls
                    .find_flow_control_with_producer_address(next)
                    .map(|x| x.flow_control_id().clone())
                {
                    // Allow a sender with corresponding flow_control_id send messages to this address
                    flow_controls.add_consumer(
                        &addresses.remote,
                        &flow_control_id,
                        FlowControlPolicy::ProducerAllowMultiple,
                    );
                }
            }
            None => {}
        }

        Ok(())
    }
}

impl Default for TcpInletTrustOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) struct ConsumerFlowControl {
    pub(super) flow_controls: FlowControls,
    pub(super) flow_control_id: FlowControlId,
    pub(super) flow_control_policy: FlowControlPolicy,
}

/// Trust Options for an Outlet
pub struct TcpOutletTrustOptions {
    pub(super) consumer_flow_control: Option<ConsumerFlowControl>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpOutletTrustOptions {
    /// Default constructor without flow control and Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_flow_control: None,
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

    /// Mark that this Outlet listener is a Consumer for to the given [`FlowControlId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`FlowControlId`]
    /// of the message that was used to create the Outlet
    pub fn as_consumer(
        mut self,
        flow_controls: &FlowControls,
        flow_control_id: &FlowControlId,
        flow_control_policy: FlowControlPolicy,
    ) -> Self {
        self.consumer_flow_control = Some(ConsumerFlowControl {
            flow_controls: flow_controls.clone(),
            flow_control_id: flow_control_id.clone(),
            flow_control_policy,
        });

        self
    }

    pub(super) fn setup_flow_control(
        &self,
        addresses: &Addresses,
        producer_flow_control_id: Option<FlowControlId>,
    ) -> Result<()> {
        match (&self.consumer_flow_control, producer_flow_control_id) {
            (Some(consumer_flow_control), Some(producer_flow_control_id)) => {
                // Allow a sender with corresponding flow_control_id send messages to this address
                consumer_flow_control.flow_controls.add_consumer(
                    &addresses.remote,
                    &producer_flow_control_id,
                    FlowControlPolicy::ProducerAllowMultiple,
                );
            }
            (None, None) => {}
            // We act as a consumer in some cases,
            // but we were reached without flow control, which is fine
            (Some(_), None) => {}
            _ => {
                return Err(TransportError::FlowControlInconsistency.into());
            }
        }

        Ok(())
    }
}

impl Default for TcpOutletTrustOptions {
    fn default() -> Self {
        Self::new()
    }
}
