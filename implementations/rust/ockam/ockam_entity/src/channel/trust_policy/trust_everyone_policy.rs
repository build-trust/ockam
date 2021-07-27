use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::{allow, Result};

#[derive(Clone)]
pub struct TrustEveryonePolicy;

impl TrustPolicy for TrustEveryonePolicy {
    fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        allow()
    }
}
