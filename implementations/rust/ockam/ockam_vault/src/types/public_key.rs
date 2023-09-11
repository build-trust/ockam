use core::fmt;

use minicbor::{Decode, Encode};
use p256::elliptic_curve::subtle;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::{PublicKeyVec, SecretType};

/// A public key.
#[derive(Encode, Decode, Serialize, Deserialize, Clone, Debug, Zeroize, PartialOrd, Ord)]
#[zeroize(drop)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PublicKey {
    #[b(1)] data: PublicKeyVec,
    #[n(2)] stype: SecretType,
}

impl Eq for PublicKey {}

impl PartialEq for PublicKey {
    fn eq(&self, o: &Self) -> bool {
        let choice = subtle::ConstantTimeEq::ct_eq(&self.data[..], &o.data[..]);
        choice.into() && self.stype == o.stype
    }
}

impl PublicKey {
    /// Public Key data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    /// Corresponding secret key type.
    pub fn stype(&self) -> SecretType {
        self.stype
    }
}

impl PublicKey {
    /// Create a new public key.
    pub fn new(data: PublicKeyVec, stype: SecretType) -> Self {
        PublicKey { data, stype }
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.stype(), hex::encode(self.data()))
    }
}
