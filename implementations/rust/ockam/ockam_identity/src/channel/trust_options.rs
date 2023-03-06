use crate::{TrustEveryonePolicy, TrustPolicy};
use ockam_core::compat::sync::Arc;
use ockam_core::sessions::{SessionId, Sessions};

/// Trust options for a Secure Channel
pub struct SecureChannelTrustOptions {
    pub(crate) ciphertext_session: Option<(Sessions, SessionId)>,
    pub(crate) _plaintext_session: Option<(Sessions, SessionId)>,
    pub(crate) trust_policy: Arc<dyn TrustPolicy>,
}

impl Default for SecureChannelTrustOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureChannelTrustOptions {
    /// Constructor
    pub fn new() -> Self {
        Self {
            ciphertext_session: None,
            _plaintext_session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Set session for ciphertext part of the Secure Channel
    pub fn with_ciphertext_session(mut self, sessions: &Sessions, session_id: &SessionId) -> Self {
        self.ciphertext_session = Some((sessions.clone(), session_id.clone()));
        self
    }

    /// Set Trust Policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }
}

/// Trust options for a Secure Channel Listener
pub struct SecureChannelListenerTrustOptions {
    pub(crate) session: Option<(Sessions, SessionId)>,
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
            session: None,
            trust_policy: Arc::new(TrustEveryonePolicy),
        }
    }

    /// Set session for this listener
    pub fn with_session(mut self, sessions: &Sessions, listener_session_id: &SessionId) -> Self {
        self.session = Some((sessions.clone(), listener_session_id.clone()));
        self
    }

    /// Set trust policy
    pub fn with_trust_policy(mut self, trust_policy: impl TrustPolicy) -> Self {
        self.trust_policy = Arc::new(trust_policy);
        self
    }

    pub(crate) fn secure_channel_trust_options(
        &self,
        session_id: Option<SessionId>,
    ) -> SecureChannelTrustOptions {
        let trust_options =
            SecureChannelTrustOptions::new().with_trust_policy(self.trust_policy.clone());

        match (&self.session, session_id) {
            // Ignore listener_session_id, since we're spawning dedicated decryptor after
            // listener received the message
            (Some((sessions, _listener_session_id)), Some(session_id)) => {
                trust_options.with_ciphertext_session(sessions, &session_id)
            }
            _ => trust_options,
        }
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
