use crate::identity::IdentityIdentifier;
use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use ockam_core::{async_trait, compat::vec::Vec, Result};
use tracing::info;

/// `TrustPolicy` based on list of pre-known `IdentityIdentifier`s of the possible participants
#[derive(Clone)]
pub struct TrustMultiIdentifiersPolicy {
    identity_ids: Vec<IdentityIdentifier>,
}

impl TrustMultiIdentifiersPolicy {
    /// Constructor
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
        if !self.contains(trust_info.their_identity_id()) {
            info!(
                "{} is not one of the trusted identifiers {}",
                trust_info.their_identity_id(),
                self.identity_ids
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
            Ok(false)
        } else {
            Ok(true)
        }
    }
}
