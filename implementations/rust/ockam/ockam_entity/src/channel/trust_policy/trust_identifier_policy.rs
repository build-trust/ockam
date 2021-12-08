use crate::{ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[derive(Clone)]
pub struct TrustIdentifierPolicy {
    their_profile_id: ProfileIdentifier,
}

impl TrustIdentifierPolicy {
    pub fn new(their_profile_id: ProfileIdentifier) -> Self {
        Self { their_profile_id }
    }
}

#[async_trait]
impl TrustPolicy for TrustIdentifierPolicy {
    async fn check(&mut self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_profile_id == self.their_profile_id)
    }
}
