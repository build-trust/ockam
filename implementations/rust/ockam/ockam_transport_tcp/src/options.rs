use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::{FlowControlId, FlowControlOutgoingAccessControl, FlowControls};
use ockam_core::{AllowAll, IncomingAccessControl, OutgoingAccessControl, Result};
use ockam_transport_core::TransportError;

pub(crate) struct TcpConnectionAccessControl {
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

/// Trust Options for a TCP connection
#[derive(Clone, Debug)]
pub struct TcpConnectionOptions {
    pub(crate) producer_flow_control: Option<(FlowControls, FlowControlId)>,
}

impl TcpConnectionOptions {
    /// This constructor is insecure, because outgoing messages from such connections will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            producer_flow_control: None,
        }
    }

    /// This constructor is insecure, because outgoing messages from such connection will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            producer_flow_control: None,
        }
    }

    /// Mark this Tcp Receivers as a Producer for a given [`FlowControlId`]
    pub fn as_producer(flow_controls: &FlowControls, flow_control_id: &FlowControlId) -> Self {
        Self {
            producer_flow_control: Some((flow_controls.clone(), flow_control_id.clone())),
        }
    }

    pub(crate) fn setup_flow_control(&self, addresses: &Addresses) {
        if let Some((flow_controls, flow_control_id)) = &self.producer_flow_control {
            flow_controls.add_producer(
                addresses.receiver_address(),
                flow_control_id,
                None,
                vec![addresses.sender_address().clone()],
            );
        }
    }

    pub(crate) fn create_access_control(self) -> TcpConnectionAccessControl {
        match self.producer_flow_control {
            Some((flow_controls, flow_control_id)) => {
                TcpConnectionAccessControl {
                    sender_incoming_access_control: Arc::new(AllowAll),
                    receiver_outgoing_access_control: Arc::new(
                        FlowControlOutgoingAccessControl::new(flow_controls, flow_control_id, None),
                    ),
                }
            }
            None => TcpConnectionAccessControl {
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(AllowAll),
            },
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Debug)]
pub struct TcpListenerOptions {
    pub(crate) spawner_flow_controls: Option<(FlowControls, FlowControlId)>,
}

impl TcpListenerOptions {
    /// This constructor is insecure, because outgoing messages from such connections will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            spawner_flow_controls: None,
        }
    }

    /// This constructor is insecure, because outgoing messages from such connections will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            spawner_flow_controls: None,
        }
    }

    /// Mark this Tcp Listener as a Spawner with given [`FlowControlId`].
    /// NOTE: Spawned connections get fresh random [`FlowControlId`], however they are still marked
    /// with Spawner's [`FlowControlId`]
    pub fn as_spawner(flow_controls: &FlowControls, flow_control_id: &FlowControlId) -> Self {
        Self {
            spawner_flow_controls: Some((flow_controls.clone(), flow_control_id.clone())),
        }
    }

    pub(crate) fn setup_flow_control(&self, addresses: &Addresses) -> Option<FlowControlId> {
        if let Some((flow_controls, listener_flow_control_id)) = &self.spawner_flow_controls {
            let flow_control_id = flow_controls.generate_id();

            flow_controls.add_producer(
                addresses.receiver_address(),
                &flow_control_id,
                Some(listener_flow_control_id),
                vec![addresses.sender_address().clone()],
            );

            Some(flow_control_id)
        } else {
            None
        }
    }

    pub(crate) fn create_access_control(
        &self,
        flow_control_id: Option<FlowControlId>,
    ) -> Result<TcpConnectionAccessControl> {
        match (&self.spawner_flow_controls, flow_control_id) {
            (Some((flow_controls, listener_flow_control_id)), Some(flow_control_id)) => {
                Ok(TcpConnectionAccessControl {
                    sender_incoming_access_control: Arc::new(AllowAll),
                    receiver_outgoing_access_control: Arc::new(
                        FlowControlOutgoingAccessControl::new(
                            flow_controls.clone(),
                            flow_control_id,
                            Some(listener_flow_control_id.clone()),
                        ),
                    ),
                })
            }
            (None, None) => Ok(TcpConnectionAccessControl {
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(AllowAll),
            }),
            _ => Err(TransportError::FlowControlInconsistency.into()),
        }
    }
}
