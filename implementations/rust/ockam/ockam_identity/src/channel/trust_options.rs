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

impl Default for SecureChannelTrustOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct SecureChannelTrustOptionsProcessed {
    pub(crate) decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    pub(crate) plaintext_session_id: Option<SessionId>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl SecureChannelTrustOptions {
    /// Constructor
    pub fn new() -> Self {
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
    ///
    /// Also this [`SessionId`] will be added to [`LocalInfo`] of the messages from that
    /// Secure Channel
    pub fn as_producer(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.producer_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn process(self, addresses: &Addresses) -> SecureChannelTrustOptionsProcessed {
        let (decryptor_outgoing_access_control, plaintext_session_id) = match self.producer_session
        {
            Some((sessions, session_id)) => {
                sessions.add_producer(&addresses.decryptor_internal, &session_id, None);

                let ac = SessionOutgoingAccessControl::new(sessions, session_id.clone(), None);

                #[allow(trivial_casts)]
                (
                    Arc::new(ac) as Arc<dyn OutgoingAccessControl>,
                    Some(session_id),
                )
            }
            None =>
            {
                #[allow(trivial_casts)]
                (Arc::new(AllowAll) as Arc<dyn OutgoingAccessControl>, None)
            }
        };

        if let Some((sessions, session_id)) = self.consumer_session {
            // Allow a sender with corresponding session_id send messages to this address
            sessions.add_consumer(
                &addresses.decryptor_remote,
                &session_id,
                SessionPolicy::ProducerAllowMultiple,
            );
        }

        SecureChannelTrustOptionsProcessed {
            decryptor_outgoing_access_control,
            plaintext_session_id,
            trust_policy: self.trust_policy,
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

impl Default for SecureChannelListenerTrustOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureChannelListenerTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self {
            consumer_session: None,
            channels_producer_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Mark that this Secure Channel Listener is a Consumer for to the given [`SessionId`]
    /// Also, in this case spawned Secure Channels will be marked as Consumers with [`SessionId`]
    /// of the message that was used to create a Secure Channel
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
    pub fn as_spawner(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.channels_producer_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn process(
        &self,
        addresses: &Addresses,
        producer_session_id: Option<SessionId>,
    ) -> Result<SecureChannelTrustOptionsProcessed> {
        let (decryptor_outgoing_access_control, plaintext_session_id) =
            match &self.channels_producer_session {
                Some((sessions, listener_session_id)) => {
                    let session_id = sessions.generate_session_id();
                    sessions.add_producer(
                        &addresses.decryptor_internal,
                        &session_id,
                        Some(listener_session_id),
                    );

                    let ac = SessionOutgoingAccessControl::new(
                        sessions.clone(),
                        session_id.clone(),
                        Some(listener_session_id.clone()),
                    );

                    #[allow(trivial_casts)]
                    (
                        Arc::new(ac) as Arc<dyn OutgoingAccessControl>,
                        Some(session_id),
                    )
                }
                None =>
                {
                    #[allow(trivial_casts)]
                    (Arc::new(AllowAll) as Arc<dyn OutgoingAccessControl>, None)
                }
            };

        match (&self.consumer_session, producer_session_id) {
            (Some(ciphertext_session), Some(producers_session_id)) => {
                // Allow a sender with corresponding session_id send messages to this address
                ciphertext_session.sessions.add_consumer(
                    &addresses.decryptor_remote,
                    &producers_session_id,
                    SessionPolicy::ProducerAllowMultiple,
                );
            }
            (None, None) => {}
            _ => {
                return Err(IdentityError::SessionsInconsistency.into());
            }
        }

        Ok(SecureChannelTrustOptionsProcessed {
            decryptor_outgoing_access_control,
            plaintext_session_id,
            trust_policy: self.trust_policy.clone(),
        })
    }
}

// Keeps backwards compatibility to allow creating secure channel by only specifying a TrustPolicy
// TODO: remove in the future to make API safer
impl<T> From<T> for SecureChannelTrustOptions
where
    T: TrustPolicy,
{
    fn from(trust_policy: T) -> Self {
        Self::new().with_trust_policy(trust_policy)
    }
}

// Keeps backwards compatibility to allow creating secure channel by only specifying a TrustPolicy
// TODO: remove in the future to make API safer
impl<T> From<T> for SecureChannelListenerTrustOptions
where
    T: TrustPolicy,
{
    fn from(trust_policy: T) -> Self {
        Self::new().with_trust_policy(trust_policy)
    }
}
