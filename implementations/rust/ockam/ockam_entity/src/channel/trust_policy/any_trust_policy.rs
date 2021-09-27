use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

#[derive(Clone)]
pub struct AnyTrustPolicy<F: TrustPolicy, S: TrustPolicy> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: TrustPolicy, S: TrustPolicy> AnyTrustPolicy<F, S> {
    pub fn new(first: F, second: S) -> Self {
        AnyTrustPolicy { first, second }
    }
}

#[async_trait]
impl<F: TrustPolicy + Sync, S: TrustPolicy + Sync> ockam_core::traits::AsyncClone
    for AnyTrustPolicy<F, S>
{
    async fn async_clone(&self) -> AnyTrustPolicy<F, S> {
        AnyTrustPolicy {
            first: self.first.async_clone().await,
            second: self.second.async_clone().await,
        }
    }
}

#[async_trait]
impl<F: TrustPolicy + Sync, S: TrustPolicy + Sync> TrustPolicy for AnyTrustPolicy<F, S> {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(self.first.check(trust_info)? || self.second.check(trust_info)?)
    }
    async fn async_check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(
            self.first.async_check(trust_info).await?
                || self.second.async_check(trust_info).await?,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{DisjunctionTrustPolicy, ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
    use ockam_core::async_trait::async_trait;
    use ockam_core::compat::boxed::Box;
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
            .or(TrustPolicyStub(true))
            .check(&trust_info)
            .unwrap());
        assert!(TrustPolicyStub(true)
            .or(TrustPolicyStub(false))
            .check(&trust_info)
            .unwrap());
        assert!(TrustPolicyStub(false)
            .or(TrustPolicyStub(true))
            .check(&trust_info)
            .unwrap());
        assert!(!TrustPolicyStub(false)
            .or(TrustPolicyStub(false))
            .check(&trust_info)
            .unwrap());
    }
}
