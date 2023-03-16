use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControl, Sessions};
use ockam_core::{
    Address, AllowAll, IncomingAccessControl, LocalOnwardOnly, LocalSourceOnly,
    OutgoingAccessControl,
};

pub(crate) struct TcpConnectionTrustOptionsProcessed {
    pub session_id: Option<SessionId>,
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

/// Trust Options for a TCP connection
#[derive(Clone, Default, Debug)]
pub struct TcpConnectionTrustOptions {
    pub(crate) producer_session: Option<(Sessions, SessionId)>,
}

impl TcpConnectionTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self {
            producer_session: None,
        }
    }

    /// Mark this Tcp Receivers as a Producer for a given [`SessionId`]
    ///
    /// Also this [`SessionId`] will be added to [`LocalInfo`] of the messages from that
    /// connection
    pub fn as_producer(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.producer_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    pub(crate) fn process(self, address: &Address) -> TcpConnectionTrustOptionsProcessed {
        match self.producer_session {
            Some((sessions, session_id)) => {
                sessions.add_producer(address, &session_id, None);

                TcpConnectionTrustOptionsProcessed {
                    session_id: Some(session_id.clone()),
                    sender_incoming_access_control: Arc::new(AllowAll),
                    receiver_outgoing_access_control: Arc::new(SessionOutgoingAccessControl::new(
                        sessions, session_id, None,
                    )),
                }
            }
            None => TcpConnectionTrustOptionsProcessed {
                session_id: None,
                sender_incoming_access_control: Arc::new(AllowAll),
                receiver_outgoing_access_control: Arc::new(AllowAll),
            },
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Default, Debug)]
pub struct TcpListenerTrustOptions {
    pub(crate) spawner_session: Option<(Sessions, SessionId)>,
}

impl TcpListenerTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self {
            spawner_session: None,
        }
    }

    /// Mark this Tcp Listener as a Spawner with given [`SessionId`].
    /// NOTE: Spawned connections get fresh random [`SessionId`], however they are still marked
    /// with Spawner's [`SessionId`]
    pub fn as_spawner(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.spawner_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    pub(crate) fn process(&self, address: &Address) -> TcpConnectionTrustOptionsProcessed {
        match &self.spawner_session {
            Some((sessions, listener_session_id)) => {
                let session_id = sessions.generate_session_id();

                sessions.add_producer(address, &session_id, Some(listener_session_id));

                TcpConnectionTrustOptionsProcessed {
                    session_id: Some(session_id.clone()),
                    sender_incoming_access_control: Arc::new(LocalSourceOnly),
                    receiver_outgoing_access_control: Arc::new(SessionOutgoingAccessControl::new(
                        sessions.clone(),
                        session_id,
                        Some(listener_session_id.clone()),
                    )),
                }
            }
            None => TcpConnectionTrustOptionsProcessed {
                session_id: None,
                sender_incoming_access_control: Arc::new(LocalSourceOnly),
                receiver_outgoing_access_control: Arc::new(LocalOnwardOnly),
            },
        }
    }
}
