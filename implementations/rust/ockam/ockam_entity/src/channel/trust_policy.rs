use crate::ProfileIdentifier;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::traits::AsyncClone;
use ockam_core::Result;
use serde::{Deserialize, Serialize};

mod trust_identifier_policy;
pub use trust_identifier_policy::*;
mod all_trust_policy;
pub use all_trust_policy::*;
mod any_trust_policy;
pub use any_trust_policy::*;
mod trust_everyone_policy;
pub use trust_everyone_policy::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct SecureChannelTrustInfo {
    their_profile_id: ProfileIdentifier,
    // TODO: credentials:
}

impl SecureChannelTrustInfo {
    pub fn their_profile_id(&self) -> &ProfileIdentifier {
        &self.their_profile_id
    }
}

impl SecureChannelTrustInfo {
    pub fn new(their_profile_id: ProfileIdentifier) -> Self {
        Self { their_profile_id }
    }
}

#[async_trait]
pub trait TrustPolicy: AsyncClone + Clone + Send + 'static {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool>;
    async fn async_check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool>;
}

pub trait ConjunctionTrustPolicy: TrustPolicy {
    fn and<O: TrustPolicy>(self, other: O) -> AllTrustPolicy<Self, O> {
        AllTrustPolicy::new(self, other)
    }
}

impl<T> ConjunctionTrustPolicy for T where T: TrustPolicy {}

pub trait DisjunctionTrustPolicy: TrustPolicy {
    fn or<O: TrustPolicy>(self, other: O) -> AnyTrustPolicy<Self, O> {
        AnyTrustPolicy::new(self, other)
    }
}

impl<T> DisjunctionTrustPolicy for T where T: TrustPolicy {}
