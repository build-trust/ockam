use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::Result;

#[derive(Clone)]
pub struct NoOpTrustPolicy;

impl TrustPolicy for NoOpTrustPolicy {
    fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(true)
    }
}
