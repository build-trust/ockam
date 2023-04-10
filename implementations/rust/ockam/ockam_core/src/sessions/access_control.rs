use crate::compat::boxed::Box;
use crate::compat::collections::BTreeSet;
use crate::compat::sync::RwLock;
use crate::sessions::{SessionId, SessionPolicy, Sessions};
use crate::{async_trait, Address, Result};
use crate::{OutgoingAccessControl, RelayMessage};

/// Session Outgoing Access Control
///
/// Allows to send messages only to members of the given [`SessionId`] or message listener
/// with given [`SessionId`]. Optionally, only 1 message can be passed to the listener.
#[derive(Debug)]
pub struct SessionOutgoingAccessControl {
    sessions: Sessions,
    session_id: SessionId,
    spawner_session_id: Option<SessionId>,
    sent_single_message_to_addresses: RwLock<BTreeSet<Address>>,
}

impl SessionOutgoingAccessControl {
    /// Constructor
    pub fn new(
        sessions: Sessions,
        session_id: SessionId,
        spawner_session_id: Option<SessionId>,
    ) -> Self {
        Self {
            sessions,
            session_id,
            spawner_session_id,
            sent_single_message_to_addresses: Default::default(),
        }
    }
}

#[async_trait]
impl OutgoingAccessControl for SessionOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> Result<bool> {
        let onward_route = relay_msg.onward_route();

        let next = onward_route.next()?;

        let session_consumers_info = self.sessions.get_consumers_info(&self.session_id);

        if let Some(policy) = session_consumers_info.0.get(next) {
            match policy {
                SessionPolicy::ProducerAllowMultiple => {
                    return crate::allow();
                }
                SessionPolicy::SpawnerAllowOnlyOneMessage => {}
                SessionPolicy::SpawnerAllowMultipleMessages => {}
            }
        }

        if let Some(spawner_session_id) = &self.spawner_session_id {
            let session_consumers_info = self.sessions.get_consumers_info(spawner_session_id);

            if let Some(policy) = session_consumers_info.0.get(next) {
                match policy {
                    SessionPolicy::SpawnerAllowOnlyOneMessage => {
                        // We haven't yet sent a message to this address
                        if !self
                            .sent_single_message_to_addresses
                            .read()
                            .unwrap()
                            .contains(next)
                        {
                            // Prevent further messages to this address
                            self.sent_single_message_to_addresses
                                .write()
                                .unwrap()
                                .insert(next.clone());

                            // Allow this message
                            return crate::allow();
                        }
                    }
                    SessionPolicy::SpawnerAllowMultipleMessages => return crate::allow(),
                    SessionPolicy::ProducerAllowMultiple => {}
                }
            }
        }

        crate::deny()
    }
}
