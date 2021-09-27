use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{allow, Result};

#[derive(Clone)]
pub struct TrustEveryonePolicy;

#[async_trait]
impl ockam_core::traits::AsyncClone for TrustEveryonePolicy {
    async fn async_clone(&self) -> TrustEveryonePolicy {
        TrustEveryonePolicy
    }
}

#[async_trait]
impl TrustPolicy for TrustEveryonePolicy {
    fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        allow()
    }
    async fn async_check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        allow()
    }
}
