use crate::portal::addresses::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionPolicy, Sessions};
use ockam_core::{AllowAll, IncomingAccessControl, Result};
use ockam_transport_core::TransportError;

/// Trust Options for an Inlet
pub struct TcpInletTrustOptions {
    pub(super) consumer_session: Option<(Sessions, SessionId)>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpInletTrustOptions {
    /// Default constructor without session and Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_session: None,
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

    /// Mark that created Inlets are Consumer for to the given [`SessionId`]
    pub fn as_consumer(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.consumer_session = Some((sessions.clone(), session_id.clone()));

        self
    }

    pub(super) fn setup_session(&self, addresses: &Addresses) -> Result<()> {
        match &self.consumer_session {
            Some((sessions, session_id)) => {
                // Allow a sender with corresponding session_id send messages to this address
                sessions.add_consumer(
                    &addresses.remote,
                    session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
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

pub(super) struct ConsumerSession {
    pub(super) sessions: Sessions,
    pub(super) session_id: SessionId,
    pub(super) session_policy: SessionPolicy,
}

/// Trust Options for an Outlet
pub struct TcpOutletTrustOptions {
    pub(super) consumer_session: Option<ConsumerSession>,
    pub(super) incoming_access_control: Arc<dyn IncomingAccessControl>,
}

impl TcpOutletTrustOptions {
    /// Default constructor without session and Incoming Access Control
    pub fn new() -> Self {
        Self {
            consumer_session: None,
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

    /// Mark that this Outlet listener is a Consumer for to the given [`SessionId`]
    /// Also, in this case spawned Outlets will be marked as Consumers with [`SessionId`]
    /// of the message that was used to create the Outlet
    pub fn as_consumer(
        mut self,
        sessions: &Sessions,
        session_id: &SessionId,
        session_policy: SessionPolicy,
    ) -> Self {
        self.consumer_session = Some(ConsumerSession {
            sessions: sessions.clone(),
            session_id: session_id.clone(),
            session_policy,
        });

        self
    }

    pub(super) fn setup_session(
        &self,
        addresses: &Addresses,
        producer_session_id: Option<SessionId>,
    ) -> Result<()> {
        match (&self.consumer_session, producer_session_id) {
            (Some(consumer_session), Some(producer_session_id)) => {
                // Allow a sender with corresponding session_id send messages to this address
                consumer_session.sessions.add_consumer(
                    &addresses.remote,
                    &producer_session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
            }
            (None, None) => {}
            _ => {
                return Err(TransportError::SessionInconsistency.into());
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
