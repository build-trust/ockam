use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Handle to a cryptographic Secret
/// Individual Vault implementations should map secret handles
/// into implementation-specific Secret representations (e.g. binaries, or HSM references)
/// stored inside Vault (e.g. using HashMap)
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Zeroize)]
pub struct Secret {
    index: usize,
}

impl Secret {
    /// Return the index of this secret.
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Secret {
    /// Create a new secret at the given index.
    pub fn new(index: usize) -> Self {
        Secret { index }
    }
}
