use crate::{KeyId, PublicKey};
use zeroize::Zeroize;

/// A key pair with a public key and a reference to the private key (which should be stored in a Vault)
#[derive(Clone, Debug, Zeroize)]
#[zeroize(drop)]
pub struct KeyPair {
    secret: KeyId,
    public: PublicKey,
}

impl KeyPair {
    /// Secret key
    pub fn secret(&self) -> &KeyId {
        &self.secret
    }
    /// Public Key
    pub fn public(&self) -> &PublicKey {
        &self.public
    }
}

impl KeyPair {
    /// Create a new key pair
    pub fn new(secret: KeyId, public: PublicKey) -> Self {
        Self { secret, public }
    }
}
