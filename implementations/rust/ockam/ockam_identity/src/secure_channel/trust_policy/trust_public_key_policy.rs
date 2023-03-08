use crate::identities::Identities;
use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::PublicKey;
use ockam_core::Result;

/// `TrustPolicy` based on pre-known `PublicKey` of the other participant
pub struct TrustPublicKeyPolicy {
    public_key: PublicKey,
    public_key_label: String,
    identities: Arc<Identities>,
}

impl TrustPublicKeyPolicy {
    /// Constructor
    pub fn new(
        public_key: PublicKey,
        public_key_label: impl Into<String>,
        identities: Arc<Identities>,
    ) -> Self {
        Self {
            public_key,
            public_key_label: public_key_label.into(),
            identities,
        }
    }
}

#[async_trait]
impl TrustPolicy for TrustPublicKeyPolicy {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let their_identity = match self
            .identities
            .identities_repository
            .get_identity(trust_info.their_identity_id())
            .await?
        {
            Some(their_identity) => their_identity,
            None => return Ok(false),
        };

        match their_identity.get_labelled_public_key(&self.public_key_label) {
            Ok(pub_key) => Ok(pub_key == self.public_key),
            _ => Ok(false),
        }
    }
}
