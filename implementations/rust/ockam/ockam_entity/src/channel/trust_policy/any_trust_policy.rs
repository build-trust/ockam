use crate::{SecureChannelTrustInfo, TrustPolicy};
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

impl<F: TrustPolicy, S: TrustPolicy> TrustPolicy for AnyTrustPolicy<F, S> {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        Ok(self.first.check(trust_info)? || self.second.check(trust_info)?)
    }
}

#[cfg(test)]
mod test {
    use crate::{DisjunctionTrustPolicy, ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy};
    use ockam_core::Result;

    #[test]
    fn test() {
        #[derive(Clone)]
        struct TrustPolicyStub(bool);

        impl TrustPolicy for TrustPolicyStub {
            fn check(&self, _trust_info: &SecureChannelTrustInfo) -> Result<bool> {
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
