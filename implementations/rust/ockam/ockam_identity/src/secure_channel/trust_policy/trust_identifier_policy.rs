use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

use crate::models::Identifier;
use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};

/// `TrustPolicy` based on pre-known `Identifier` of the other participant
#[derive(Clone)]
pub struct TrustIdentifierPolicy {
    their_identity_id: Identifier,
}

impl TrustIdentifierPolicy {
    /// Constructor
    pub fn new(their_identity_id: Identifier) -> Self {
        Self { their_identity_id }
    }
}

#[async_trait]
impl TrustPolicy for TrustIdentifierPolicy {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_identity_id == self.their_identity_id)
    }
}
