use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

#[derive(Clone)]
pub struct AllTrustPolicy<F: TrustPolicy, S: TrustPolicy> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: TrustPolicy, S: TrustPolicy> AllTrustPolicy<F, S> {
    pub fn new(first: F, second: S) -> Self {
        AllTrustPolicy { first, second }
    }
}

#[async_trait]
impl<F: TrustPolicy + Sync, S: TrustPolicy + Sync> ockam_core::traits::AsyncClone
    for AllTrustPolicy<F, S>
{
    async fn async_clone(&self) -> AllTrustPolicy<F, S> {
        AllTrustPolicy {
            first: self.first.async_clone().await,
            second: self.second.async_clone().await,
        }
    }
}

#[async_trait]
impl<F: TrustPolicy + Sync, S: TrustPolicy + Sync> TrustPolicy for AllTrustPolicy<F, S> {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(self.first.check(trust_info)? && self.second.check(trust_info)?)
    }
    async fn async_check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(
            self.first.async_check(trust_info).await?
                && self.second.async_check(trust_info).await?,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ConjunctionTrustPolicy, ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
    use ockam_core::{traits::AsyncClone, Result};

    #[test]
    fn test() {
        #[derive(Clone)]
        struct TrustPolicyStub(bool);

        #[async_trait]
        impl AsyncClone for TrustPolicyStub {
            async fn async_clone(&self) -> TrustPolicyStub {
                self.clone()
            }
        }

        #[async_trait]
        impl TrustPolicy for TrustPolicyStub {
            fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
                Ok(self.0)
            }
            async fn async_check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
                Ok(self.0)
            }
        }

        let id = ProfileIdentifier::random();
        let trust_info = SecureChannelTrustInfo::new(id);

        assert!(TrustPolicyStub(true)
            .and(TrustPolicyStub(true))
            .check(&trust_info)
            .unwrap());
        assert!(!TrustPolicyStub(true)
            .and(TrustPolicyStub(false))
            .check(&trust_info)
            .unwrap());
        assert!(!TrustPolicyStub(false)
            .and(TrustPolicyStub(true))
            .check(&trust_info)
            .unwrap());
        assert!(!TrustPolicyStub(false)
            .and(TrustPolicyStub(false))
            .check(&trust_info)
            .unwrap());
    }
}
