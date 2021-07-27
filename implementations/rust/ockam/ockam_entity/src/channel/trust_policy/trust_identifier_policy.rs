use crate::{ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::Result;

#[derive(Clone)]
pub struct TrustIdentifierPolicy {
    their_profile_id: ProfileIdentifier,
}

impl TrustIdentifierPolicy {
    pub fn new(their_profile_id: ProfileIdentifier) -> Self {
        Self { their_profile_id }
    }
}

impl TrustPolicy for TrustIdentifierPolicy {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_profile_id == self.their_profile_id)
    }
}
