use crate::{IdentityIdentifier, SecureChannelTrustInfo, TrustPolicy};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, vec::Vec},
    Result,
};

#[derive(Clone)]
pub struct TrustMultiIdentifiersPolicy {
    identity_ids: Vec<IdentityIdentifier>,
}

impl TrustMultiIdentifiersPolicy {
    pub fn new(identity_ids: Vec<IdentityIdentifier>) -> Self {
        Self { identity_ids }
    }

    fn contains(&self, their_id: &IdentityIdentifier) -> bool {
        let mut found = subtle::Choice::from(0);
        for trusted_id in &*self.identity_ids {
            found |= trusted_id.ct_eq(their_id);
        }
        found.into()
    }
}

#[async_trait]
impl TrustPolicy for TrustMultiIdentifiersPolicy {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(self.contains(trust_info.their_identity_id()))
    }
}
