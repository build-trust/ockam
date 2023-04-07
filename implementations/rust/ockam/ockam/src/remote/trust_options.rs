use crate::remote::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControl, SessionPolicy, Sessions};
use ockam_core::{Address, AllowAll, OutgoingAccessControl};

/// Trust options for [`RemoteForwarder`]
pub struct RemoteForwarderTrustOptions {
    pub(super) sessions: Option<Sessions>,
}

impl RemoteForwarderTrustOptions {
    /// This constructor is insecure, because outgoing messages from such forwarder will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure_test() -> Self {
        Self { sessions: None }
    }

    /// Mark this [`RemoteForwarder`] as a Producer and Consumer for a given [`SessionId`]
    /// Usually [`SessionId`] should be shared with the Producer that was used to create this
    /// forwarder (probably Secure Channel), since [`RemoteForwarder`] doesn't imply any new "trust"
    /// context, it's just a Message Routing helper. Therefore, workers that are allowed to receive
    /// messages from the corresponding Secure Channel should as well be allowed to receive messages
    /// through the [`RemoteForwarder`] through the same Secure Channel.
    pub fn as_consumer_and_producer(sessions: &Sessions) -> Self {
        Self {
            sessions: Some(sessions.clone()),
        }
    }

    pub(super) fn setup_session(&self, addresses: &Addresses, next: &Address) -> Option<SessionId> {
        match &self.sessions {
            Some(sessions) => {
                match sessions
                    .find_session_with_producer_address(next)
                    .map(|x| x.session_id().clone())
                {
                    Some(session_id) => {
                        // Allow a sender with corresponding session_id send messages to this address
                        sessions.add_consumer(
                            &addresses.main_remote,
                            &session_id,
                            SessionPolicy::ProducerAllowMultiple,
                        );

                        sessions.add_producer(&addresses.main_internal, &session_id, None, vec![]);

                        Some(session_id)
                    }
                    None => None,
                }
            }
            None => None,
        }
    }

    pub(super) fn create_access_control(
        &self,
        session_id: Option<SessionId>,
    ) -> Arc<dyn OutgoingAccessControl> {
        match &self.sessions {
            Some(sessions) => {
                let ac = SessionOutgoingAccessControl::new(
                    sessions.clone(),
                    session_id.unwrap().clone(),
                    None,
                );

                Arc::new(ac)
            }
            None => Arc::new(AllowAll),
        }
    }
}
