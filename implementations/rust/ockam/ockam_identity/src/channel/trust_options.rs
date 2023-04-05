use crate::channel::addresses::Addresses;
use crate::error::IdentityError;
use crate::{TrustEveryonePolicy, TrustPolicy};
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, SessionOutgoingAccessControl, SessionPolicy, Sessions};
use ockam_core::{AllowAll, OutgoingAccessControl, Result};

/// Trust options for a Secure Channel
pub struct SecureChannelTrustOptions {
    pub(crate) consumer_session: Option<(Sessions, SessionId)>,
    pub(crate) producer_session: Option<(Sessions, SessionId)>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

pub(crate) struct SecureChannelAccessControl {
    pub(crate) decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
}

impl SecureChannelTrustOptions {
    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            consumer_session: None,
            producer_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure_test() -> Self {
        Self {
            consumer_session: None,
            producer_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Mark this Secure Channel Decryptor as a Consumer for a given [`SessionId`]
    pub fn as_consumer(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.consumer_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    /// Mark this Secure Channel Decryptor as a Producer for a given [`SessionId`]
    pub fn as_producer(sessions: &Sessions, session_id: &SessionId) -> Self {
        Self {
            consumer_session: None,
            producer_session: Some((sessions.clone(), session_id.clone())),
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn setup_session(&self, addresses: &Addresses) {
        if let Some((sessions, session_id)) = &self.consumer_session {
            // Allow a sender with corresponding session_id send messages to this address
            sessions.add_consumer(
                &addresses.decryptor_remote,
                session_id,
                SessionPolicy::ProducerAllowMultiple,
            );
        }

        if let Some((sessions, session_id)) = &self.producer_session {
            sessions.add_producer(
                &addresses.decryptor_internal,
                session_id,
                None,
                vec![addresses.encryptor.clone()],
            );
        }
    }

    pub(crate) fn create_access_control(&self) -> SecureChannelAccessControl {
        match &self.producer_session {
            Some((sessions, session_id)) => {
                let ac =
                    SessionOutgoingAccessControl::new(sessions.clone(), session_id.clone(), None);

                SecureChannelAccessControl {
                    decryptor_outgoing_access_control: Arc::new(ac),
                }
            }
            None => SecureChannelAccessControl {
                decryptor_outgoing_access_control: Arc::new(AllowAll),
            },
        }
    }
}

pub(crate) struct CiphertextSession {
    pub(crate) sessions: Sessions,
    pub(crate) session_id: SessionId,
    pub(crate) session_policy: SessionPolicy,
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerTrustOptions {
    pub(crate) consumer_session: Option<CiphertextSession>,
    pub(crate) channels_producer_session: Option<(Sessions, SessionId)>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl SecureChannelListenerTrustOptions {
    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure() -> Self {
        Self {
            consumer_session: None,
            channels_producer_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// This constructor is insecure, because outgoing messages from such channels will not be
    /// restricted and can reach any [`Address`] on this node.
    /// Should only be used for testing purposes
    pub fn insecure_test() -> Self {
        Self {
            consumer_session: None,
            channels_producer_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Mark that this Secure Channel Listener is a Consumer for to the given [`SessionId`]
    /// Also, in this case spawned Secure Channels will be marked as Consumers with [`SessionId`]
    /// of the message that was used to create the Secure Channel
    pub fn as_consumer(
        mut self,
        sessions: &Sessions,
        session_id: &SessionId,
        session_policy: SessionPolicy,
    ) -> Self {
        self.consumer_session = Some(CiphertextSession {
            sessions: sessions.clone(),
            session_id: session_id.clone(),
            session_policy,
        });

        self
    }

    /// Mark spawned Secure Channel Decryptors as Producers for a given Spawner's [`SessionId`]
    /// NOTE: Spawned connections get fresh random [`SessionId`], however they are still marked
    /// with Spawner's [`SessionId`]
    pub fn as_spawner(sessions: &Sessions, session_id: &SessionId) -> Self {
        Self {
            consumer_session: None,
            channels_producer_session: Some((sessions.clone(), session_id.clone())),
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn setup_session(
        &self,
        addresses: &Addresses,
        producer_session_id: Option<SessionId>,
    ) -> Result<Option<SessionId>> {
        match (&self.consumer_session, producer_session_id) {
            (Some(ciphertext_session), Some(producer_session_id)) => {
                // Allow a sender with corresponding session_id send messages to this address
                ciphertext_session.sessions.add_consumer(
                    &addresses.decryptor_remote,
                    &producer_session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
            }
            (None, None) => {}
            _ => {
                return Err(IdentityError::SessionsInconsistency.into());
            }
        }

        match &self.channels_producer_session {
            Some((sessions, listener_session_id)) => {
                let session_id = sessions.generate_session_id();
                sessions.add_producer(
                    &addresses.decryptor_internal,
                    &session_id,
                    Some(listener_session_id),
                    vec![addresses.encryptor.clone()],
                );

                Ok(Some(session_id))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn create_access_control(
        &self,
        session_id: Option<SessionId>,
    ) -> Result<SecureChannelAccessControl> {
        match (&self.channels_producer_session, session_id) {
            (Some((sessions, listener_session_id)), Some(session_id)) => {
                let ac = SessionOutgoingAccessControl::new(
                    sessions.clone(),
                    session_id,
                    Some(listener_session_id.clone()),
                );

                Ok(SecureChannelAccessControl {
                    decryptor_outgoing_access_control: Arc::new(ac),
                })
            }
            (None, None) => Ok(SecureChannelAccessControl {
                decryptor_outgoing_access_control: Arc::new(AllowAll),
            }),
            _ => Err(IdentityError::SessionsInconsistency.into()),
        }
    }
}
