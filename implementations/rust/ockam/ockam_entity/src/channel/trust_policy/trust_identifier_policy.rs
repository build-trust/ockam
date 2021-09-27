use crate::{ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
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

#[async_trait]
impl ockam_core::traits::AsyncClone for TrustIdentifierPolicy {
    async fn async_clone(&self) -> TrustIdentifierPolicy {
        TrustIdentifierPolicy {
            their_profile_id: self.their_profile_id.clone(),
        }
    }
}

#[async_trait]
impl TrustPolicy for TrustIdentifierPolicy {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_profile_id == self.their_profile_id)
    }
    async fn async_check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(trust_info.their_profile_id == self.their_profile_id)
    }
}
