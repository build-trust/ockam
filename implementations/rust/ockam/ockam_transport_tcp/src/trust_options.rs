use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControlBuilder, Sessions};
use ockam_core::{IncomingAccessControl, LocalOnwardOnly, LocalSourceOnly, OutgoingAccessControl};

pub(crate) struct TcpConnectionAccessControl {
    pub session_id: Option<SessionId>,
    pub sender_incoming_access_control: Arc<dyn IncomingAccessControl>,
    pub receiver_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

/// Trust Options for a TCP connection
#[derive(Clone, Default, Debug)]
pub struct TcpConnectionTrustOptions {
    pub(crate) session: Option<(Sessions, SessionId)>,
}

impl TcpConnectionTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self { session: None }
    }

    /// Set session for this connection, in this case messages from that connection
    /// will be only allowed to go to the [`Address`]es with the same [`SessionId`].
    /// Information of [`Address`]' [`SessionId`] is stored in [`Sessions`] struct.
    ///
    /// Also this [`SessionId`] will be added to [`LocalInfo`] of the messages from that
    /// connection
    pub fn with_session(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.session = Some((sessions.clone(), session_id.clone()));
        self
    }

    pub(crate) fn access_control(self) -> TcpConnectionAccessControl {
        match self.session {
            Some((sessions, session_id)) => TcpConnectionAccessControl {
                session_id: Some(session_id.clone()),
                sender_incoming_access_control: Arc::new(LocalSourceOnly),
                receiver_outgoing_access_control: Arc::new(
                    SessionOutgoingAccessControlBuilder::new(session_id, sessions).build(),
                ),
            },
            None => TcpConnectionAccessControl {
                session_id: None,
                sender_incoming_access_control: Arc::new(LocalSourceOnly),
                receiver_outgoing_access_control: Arc::new(LocalOnwardOnly),
            },
        }
    }
}

/// Trust Options for a TCP listener
#[derive(Default, Debug)]
pub struct TcpListenerTrustOptions {
    pub(crate) session: Option<(Sessions, SessionId)>,
}

impl TcpListenerTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self { session: None }
    }

    /// Set session for this listener, in this case messages from connections have following
    /// outgoing access control:
    ///  - 1 message is allowed to the [`Address`]
    ///     with the same listener [`SessionId`] (e.g., SecureChannel listener)
    /// - fresh [`SessionId`] is generated for each spawned connection
    /// - messages are allowed to the [`Address`]es with the same fresh [`SessionId`]
    /// - fresh [`SessionId`] is added to the [`LocalInfo`] of messages
    ///     received by this TCP connection
    ///
    /// Information of [`Address`]' [`SessionId`] is stored in [`Sessions`] struct.
    pub fn with_session(mut self, sessions: &Sessions, listener_session_id: &SessionId) -> Self {
        self.session = Some((sessions.clone(), listener_session_id.clone()));
        self
    }

    pub(crate) fn access_control(&self) -> TcpConnectionAccessControl {
        match &self.session {
            Some((sessions, listener_session_id)) => {
                let session_id = sessions.generate_session_id();
                TcpConnectionAccessControl {
                    session_id: Some(session_id.clone()),
                    sender_incoming_access_control: Arc::new(LocalSourceOnly),
                    receiver_outgoing_access_control: Arc::new(
                        SessionOutgoingAccessControlBuilder::new(session_id, sessions.clone())
                            .allow_one_message_to_the_listener(listener_session_id.clone())
                            .build(),
                    ),
                }
            }
            None => TcpConnectionAccessControl {
                session_id: None,
                sender_incoming_access_control: Arc::new(LocalSourceOnly),
                receiver_outgoing_access_control: Arc::new(LocalOnwardOnly),
            },
        }
    }
}
