use crate::workers::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControl, Sessions};
use ockam_core::{Address, AllowAll, IncomingAccessControl, OutgoingAccessControl, Result};
use ockam_transport_core::TransportError;

pub(crate) struct TcpConnectionAccessControl {
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

/// Trust Options for a TCP connection
#[derive(Clone, Debug)]
pub struct TcpConnectionTrustOptions {
    pub(crate) producer_session: Option<(Sessions, SessionId)>,
}

impl TcpConnectionTrustOptions {
    /// This constructor is insecure, because outgoing messages from such connection will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            producer_session: None,
        }
    }

    /// This constructor is insecure, because outgoing messages from such connection will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure_test() -> Self {
        Self {
            producer_session: None,
        }
    }

    /// Mark this Tcp Receivers as a Producer for a given [`SessionId`]
    pub fn as_producer(sessions: &Sessions, session_id: &SessionId) -> Self {
        Self {
            producer_session: Some((sessions.clone(), session_id.clone())),
        }
    }

    pub(crate) fn setup_session(&self, addresses: &Addresses) {
        if let Some((sessions, session_id)) = &self.producer_session {
            sessions.add_producer(
                addresses.receiver_address(),
                session_id,
                None,
                vec![addresses.sender_address().clone()],
            );
        }
    }

    pub(crate) fn create_access_control(self) -> TcpConnectionAccessControl {
        match self.producer_session {
            Some((sessions, session_id)) => TcpConnectionAccessControl {
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(SessionOutgoingAccessControl::new(
                    sessions, session_id, None,
                )),
            },
            None => TcpConnectionAccessControl {
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(AllowAll),
            },
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Debug)]
pub struct TcpListenerTrustOptions {
    pub(crate) spawner_session: Option<(Sessions, SessionId)>,
}

impl TcpListenerTrustOptions {
    /// This constructor is insecure, because outgoing messages from such connections will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            spawner_session: None,
        }
    }

    /// This constructor is insecure, because outgoing messages from such connections will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure_test() -> Self {
        Self {
            spawner_session: None,
        }
    }

    /// Mark this Tcp Listener as a Spawner with given [`SessionId`].
    /// NOTE: Spawned connections get fresh random [`SessionId`], however they are still marked
    /// with Spawner's [`SessionId`]
    pub fn as_spawner(sessions: &Sessions, session_id: &SessionId) -> Self {
        Self {
            spawner_session: Some((sessions.clone(), session_id.clone())),
        }
    }

    pub(crate) fn setup_session(&self, address: &Address) -> Option<SessionId> {
        if let Some((sessions, listener_session_id)) = &self.spawner_session {
            let session_id = sessions.generate_session_id();

            sessions.add_producer(address, &session_id, Some(listener_session_id), vec![]);

            Some(session_id)
        } else {
            None
        }
    }

    pub(crate) fn create_access_control(
        &self,
        session_id: Option<SessionId>,
    ) -> Result<TcpConnectionAccessControl> {
        match (&self.spawner_session, session_id) {
            (Some((sessions, listener_session_id)), Some(session_id)) => {
                Ok(TcpConnectionAccessControl {
                    sender_incoming_access_control: Arc::new(AllowAll),
                    receiver_outgoing_access_control: Arc::new(SessionOutgoingAccessControl::new(
                        sessions.clone(),
                        session_id,
                        Some(listener_session_id.clone()),
                    )),
                })
            }
            (None, None) => Ok(TcpConnectionAccessControl {
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(AllowAll),
            }),
            _ => Err(TransportError::SessionInconsistency.into()),
        }
    }
}
