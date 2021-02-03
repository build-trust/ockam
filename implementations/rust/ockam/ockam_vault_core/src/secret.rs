use zeroize::Zeroize;

/// Handle to any cryptographic Secret
/// Individual Vault implementations should map secret handles
/// into implementation-specific Secret representations (e.g. binaries, or HSM references)
/// stored inside Vault (e.g. using HashMap)
#[derive(Clone, Debug, Zeroize)]
pub struct Secret {
    index: usize,
}

impl Secret {
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Secret {
    pub fn new(index: usize) -> Self {
        Secret { index }
    }
}
