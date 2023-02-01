use crate::authenticated_storage::AuthenticatedStorage;
use crate::{Identity, IdentityVault, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::compat::string::String;
use ockam_core::vault::PublicKey;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};

/// `TrustPolicy` based on pre-known `PublicKey` of the other participant
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct TrustPublicKeyPolicy<V: IdentityVault, S: AuthenticatedStorage> {
    public_key: PublicKey,
    public_key_label: String,
    identity: Identity<V, S>,
}

impl<V: IdentityVault, S: AuthenticatedStorage> TrustPublicKeyPolicy<V, S> {
    /// Constructor
    pub fn new(
        public_key: PublicKey,
        public_key_label: impl Into<String>,
        identity: Identity<V, S>,
    ) -> Self {
        Self {
            public_key,
            public_key_label: public_key_label.into(),
            identity,
        }
    }
}

#[async_trait]
impl<V: IdentityVault, S: AuthenticatedStorage> TrustPolicy for TrustPublicKeyPolicy<V, S> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let their_identity = match self
            .identity
            .get_known_identity(trust_info.their_identity_id())
            .await?
        {
            Some(their_identity) => their_identity,
            None => return Ok(false),
        };

        match their_identity.get_public_key(&self.public_key_label) {
            Ok(pub_key) => Ok(pub_key == self.public_key),
            _ => Ok(false),
        }
    }
}
