use crate::identity::IdentityIdentifier;
use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

/// `TrustPolicy` based on pre-known `IdentityIdentifier` of the other participant
#[derive(Clone)]
pub struct TrustIdentifierPolicy {
    their_identity_id: IdentityIdentifier,
}

impl TrustIdentifierPolicy {
    /// Constructor
    pub fn new(their_identity_id: IdentityIdentifier) -> Self {
        Self { their_identity_id }
    }
}

#[async_trait]
impl TrustPolicy for TrustIdentifierPolicy {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_identity_id == self.their_identity_id)
    }
}
