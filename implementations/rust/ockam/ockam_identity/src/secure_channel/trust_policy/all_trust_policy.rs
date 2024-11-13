use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{Result, TryClone};

use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};

/// Succeeds only if both `TrustPolicy` checks succeeded
#[derive(TryClone)]
#[try_clone(crate = "ockam_core")]
pub struct AllTrustPolicy<F: TrustPolicy, S: TrustPolicy> {
    // TODO: Extend for more than 2 policies
    first: F,
    second: S,
}

impl<F: TrustPolicy, S: TrustPolicy> AllTrustPolicy<F, S> {
    /// Constructor
    pub fn new(first: F, second: S) -> Self {
        AllTrustPolicy { first, second }
    }
}

#[async_trait]
impl<F: TrustPolicy, S: TrustPolicy> TrustPolicy for AllTrustPolicy<F, S> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(self.first.check(trust_info).await? && self.second.check(trust_info).await?)
    }
}

#[cfg(test)]
mod test {
    use crate::models::Identifier;
    use crate::secure_channel::trust_policy::{SecureChannelTrustInfo, TrustPolicy};
    use ockam_core::async_trait;
    use ockam_core::Result;

    #[tokio::test]
    async fn test() {
        #[derive(Clone)]
        struct TrustPolicyStub(bool);

        #[async_trait]
        impl TrustPolicy for TrustPolicyStub {
            async fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
                Ok(self.0)
            }
        }

        let id = Identifier::try_from(
            "Iabababababababababababababababababababababababababababababababab",
        )
        .unwrap();
        let trust_info = SecureChannelTrustInfo::new(id);

        assert!(TrustPolicyStub(true)
            .and(TrustPolicyStub(true))
            .check(&trust_info)
            .await
            .unwrap());
        assert!(!TrustPolicyStub(true)
            .and(TrustPolicyStub(false))
            .check(&trust_info)
            .await
            .unwrap());
        assert!(!TrustPolicyStub(false)
            .and(TrustPolicyStub(true))
            .check(&trust_info)
            .await
            .unwrap());
        assert!(!TrustPolicyStub(false)
            .and(TrustPolicyStub(false))
            .check(&trust_info)
            .await
            .unwrap());
    }
}
