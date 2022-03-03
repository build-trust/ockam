use crate::IdentityIdentifier;
use ockam_core::{
    async_trait,
    compat::{boxed::Box, sync::Arc},
    Result,
};
use serde::{Deserialize, Serialize};

mod trust_identifier_policy;
pub use trust_identifier_policy::*;
mod trust_multi_identifier_policy;
pub use trust_multi_identifier_policy::*;
mod all_trust_policy;
pub use all_trust_policy::*;
mod any_trust_policy;
pub use any_trust_policy::*;
mod trust_everyone_policy;
pub use trust_everyone_policy::*;
mod trust_public_key_policy;
pub use trust_public_key_policy::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct SecureChannelTrustInfo {
    their_identity_id: IdentityIdentifier,
    // TODO: credentials:
}

impl SecureChannelTrustInfo {
    pub fn their_identity_id(&self) -> &IdentityIdentifier {
        &self.their_identity_id
    }
}

impl SecureChannelTrustInfo {
    pub fn new(their_identity_id: IdentityIdentifier) -> Self {
        Self { their_identity_id }
    }
}

#[async_trait]
pub trait TrustPolicy: Send + Sync + 'static {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool>;

    fn and<O: TrustPolicy>(self, other: O) -> AllTrustPolicy<Self, O>
    where
        Self: Sized,
    {
        AllTrustPolicy::new(self, other)
    }

    fn or<O: TrustPolicy>(self, other: O) -> AnyTrustPolicy<Self, O>
    where
        Self: Sized,
    {
        AnyTrustPolicy::new(self, other)
    }
}

// Allow `Box<dyn TrustPolicy>` to be used as a valid TrustPolicy
#[async_trait]
impl<T: TrustPolicy + ?Sized> TrustPolicy for Box<T> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        T::check(&**self, trust_info).await
    }
}

#[async_trait]
impl<T: TrustPolicy + ?Sized> TrustPolicy for Arc<T> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        T::check(&**self, trust_info).await
    }
}
