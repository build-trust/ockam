use crate::SignatureVec;
use p256::elliptic_curve::subtle;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Binary representation of Signature.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize)]
pub struct Signature(SignatureVec);

impl Signature {
    /// Create a new signature.
    pub fn new(data: SignatureVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Eq for Signature {}

impl PartialEq for Signature {
    fn eq(&self, o: &Self) -> bool {
        subtle::ConstantTimeEq::ct_eq(&self.0[..], &o.0[..]).into()
    }
}

impl From<Signature> for SignatureVec {
    fn from(sig: Signature) -> Self {
        sig.0
    }
}
