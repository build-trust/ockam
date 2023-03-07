use crate::compat::boxed::Box;
use crate::compat::sync::RwLock;
use crate::sessions::{SessionId, Sessions};
use crate::Result;
use crate::{OutgoingAccessControl, RelayMessage};

/// Builder for [`SessionOutgoingAccessControl`]
pub struct SessionOutgoingAccessControlBuilder {
    session_id: SessionId,
    listener_session_id: Option<SessionId>,
    allow_one_message_to_the_listener: bool,
    sessions: Sessions,
}

impl SessionOutgoingAccessControlBuilder {
    /// Constructor
    pub fn new(session_id: SessionId, sessions: Sessions) -> Self {
        Self {
            session_id,
            listener_session_id: None,
            allow_one_message_to_the_listener: true,
            sessions,
        }
    }

    /// Consume the builder and build [`SessionOutgoingAccessControl`]
    pub fn build(self) -> SessionOutgoingAccessControl {
        SessionOutgoingAccessControl::new(
            self.session_id,
            self.listener_session_id,
            self.allow_one_message_to_the_listener,
            self.sessions,
        )
    }

    /// Allow one message to the listener with given listener session id (this will
    /// make listener to spawn a new session)
    pub fn allow_one_message_to_the_listener(mut self, listener_session_id: SessionId) -> Self {
        self.listener_session_id = Some(listener_session_id);
        self.allow_one_message_to_the_listener = true;

        self
    }
}

/// Session Outgoing Access Control
///
/// Allows to send messages only to members of the given [`SessionId`] or message listener
/// with given [`SessionId`]. Optionally, only 1 message can be passed to the listener.
#[derive(Debug)]
pub struct SessionOutgoingAccessControl {
    session_id: SessionId,
    listener_session_id: Option<SessionId>,
    allow_only_1_message_to_the_listener: bool,
    message_was_sent_to_the_listener: RwLock<bool>,
    sessions: Sessions,
}

impl SessionOutgoingAccessControl {
    /// Constructor
    pub fn new(
        session_id: SessionId,
        listener_session_id: Option<SessionId>,
        allow_only_1_message_to_the_listener: bool,
        sessions: Sessions,
    ) -> Self {
        Self {
            session_id,
            listener_session_id,
            allow_only_1_message_to_the_listener,
            message_was_sent_to_the_listener: RwLock::new(false),
            sessions,
        }
    }
}

#[async_trait]
impl OutgoingAccessControl for SessionOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        let next = onward_route.next()?;

        // Allow messages to workers with the same session id
        if let Some(address_session_id) = self.sessions.get_session_id(next) {
            if address_session_id == self.session_id {
                return crate::allow();
            }
        }

        // Allow messages to the corresponding listener
        if let (Some(listener_session_id), Some(address_listener_session_id)) = (
            &self.listener_session_id,
            self.sessions.get_listener_session_id(next),
        ) {
            // This is the intended listener
            #[allow(clippy::collapsible_if)]
            if listener_session_id == &address_listener_session_id {
                // We haven't yet sent a message to it
                if !self.allow_only_1_message_to_the_listener
                    || !*self.message_was_sent_to_the_listener.read().unwrap()
                {
                    // Prevent further message to it
                    *self.message_was_sent_to_the_listener.write().unwrap() = true;
                    return crate::allow();
                }
            }
        }

        crate::deny()
    }
}
