use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::{allow, Result};
use ockam_core::{async_trait, compat::boxed::Box};

/// Trust any participant
#[derive(Clone)]
pub struct TrustEveryonePolicy;

#[async_trait]
impl TrustPolicy for TrustEveryonePolicy {
    async fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        allow()
    }
}
