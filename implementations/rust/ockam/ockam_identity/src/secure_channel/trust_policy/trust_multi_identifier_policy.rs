use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::String;
use ockam_core::compat::string::ToString;
use ockam_core::{async_trait, compat::vec::Vec, Result};
use tracing::info;

use crate::models::Identifier;
use crate::trust_policy::{SecureChannelTrustInfo, TrustPolicy};

/// `TrustPolicy` based on list of pre-known `Identifier`s of the possible participants
#[derive(Clone)]
pub struct TrustMultiIdentifiersPolicy {
    identity_ids: Vec<Identifier>,
}

impl TrustMultiIdentifiersPolicy {
    /// Constructor
    pub fn new(identity_ids: Vec<Identifier>) -> Self {
        Self { identity_ids }
    }
}

#[async_trait]
impl TrustPolicy for TrustMultiIdentifiersPolicy {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        if !self.identity_ids.contains(trust_info.their_identity_id()) {
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
