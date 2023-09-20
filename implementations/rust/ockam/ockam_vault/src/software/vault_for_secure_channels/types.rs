use cfg_if::cfg_if;
use zeroize::{Zeroize, ZeroizeOnDrop};

use ockam_core::compat::vec::Vec;

/// X25519 private key length.
pub const X25519_SECRET_KEY_LENGTH: usize = 32;

/// X25519 Secret Key.
#[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct X25519SecretKey([u8; X25519_SECRET_KEY_LENGTH]);

impl X25519SecretKey {
    /// Constructor.
    pub fn new(key: [u8; X25519_SECRET_KEY_LENGTH]) -> Self {
        Self(key)
    }

    pub(crate) fn key(&self) -> &[u8; X25519_SECRET_KEY_LENGTH] {
        &self.0
    }
}

/// Buffer with sensitive data, like HKDF output.
#[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct BufferSecret(Vec<u8>);

impl BufferSecret {
    /// Constructor.
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub(crate) fn data(&self) -> &[u8] {
        self.0.as_slice()
    }
}

cfg_if! {
    if #[cfg(any(not(feature = "disable_default_noise_protocol"), feature = "OCKAM_XX_25519_AES256_GCM_SHA256"))] {
        /// AES256 private key length.
        pub const AES256_SECRET_LENGTH: usize = 32;

        /// AEAD Secret Length.
        pub const AEAD_SECRET_LENGTH: usize = AES256_SECRET_LENGTH;

        /// AES-GCM nonce length
        pub const AES_NONCE_LENGTH: usize = 12;

        /// AEAD Secret.
        #[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
        pub struct AeadSecret(pub [u8; AEAD_SECRET_LENGTH]);
    } else if #[cfg(feature = "OCKAM_XX_25519_AES128_GCM_SHA256")] {
        /// AES128 private key length.
        pub const AES128_SECRET_LENGTH: usize = 16;

        /// AEAD Secret Length.
        pub const AEAD_SECRET_LENGTH: usize = AES128_SECRET_LENGTH;

        /// AES-GCM nonce length
        pub const AES_NONCE_LENGTH: usize = 12;

        /// AEAD Secret.
        #[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
        pub struct AeadSecret(pub [u8; AEAD_SECRET_LENGTH]);

    } else if #[cfg(feature = "OCKAM_XX_25519_ChaChaPolyBLAKE2s")] {
        // TODO
    }
}
