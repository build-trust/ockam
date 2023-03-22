use crate::remote::Addresses;
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControl, SessionPolicy, Sessions};
use ockam_core::{AllowAll, OutgoingAccessControl};

/// Trust options for [`RemoteForwarder`]
#[derive(Default)]
pub struct RemoteForwarderTrustOptions {
    pub(super) session: Option<(Sessions, SessionId)>,
}

impl RemoteForwarderTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self { session: None }
    }

    /// Mark this Secure Channel Decryptor as a Producer and Producer for a given [`SessionId`]
    /// Usually [`SessionId`] should be shared with the Producer that was used to create this
    /// forwarder (probably Secure Channel), since RemoteForwarder doesn't imply a new "trust"
    /// Context, it's just a Message Routing helper. Therefore, workers that are allowed to receive
    /// messages from the corresponding Secure Channel should as well be allowed to receive messages
    /// through the RemoteForwarder through the same Secure Channel.
    pub fn as_consumer_and_producer(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.session = Some((sessions.clone(), session_id.clone()));
        self
    }

    pub(super) fn setup_session(&self, addresses: &Addresses) {
        if let Some((sessions, session_id)) = &self.session {
            // Allow a sender with corresponding session_id send messages to this address
            sessions.add_consumer(
                &addresses.main_remote,
                session_id,
                SessionPolicy::ProducerAllowMultiple,
            );

            sessions.add_producer(&addresses.main_internal, session_id, None);
        }
    }

    pub(super) fn create_access_control(&self) -> Arc<dyn OutgoingAccessControl> {
        match &self.session {
            Some((sessions, session_id)) => {
                let ac =
                    SessionOutgoingAccessControl::new(sessions.clone(), session_id.clone(), None);

                Arc::new(ac)
            }
            None => Arc::new(AllowAll),
        }
    }
}
