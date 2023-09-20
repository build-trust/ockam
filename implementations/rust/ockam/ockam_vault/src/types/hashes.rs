use cfg_if::cfg_if;

use crate::{HandleToSecret, SecretBufferHandle};
use ockam_core::compat::vec::Vec;

/// SHA256 digest length
pub const SHA256_LENGTH: usize = 32;

/// SHA-256 Output.
pub struct Sha256Output(pub [u8; SHA256_LENGTH]);

cfg_if! {
    if #[cfg(any(not(feature = "disable_default_noise_protocol"), feature = "OCKAM_XX_25519_AES256_GCM_SHA256"))] {
        /// Hash used for Noise handshake.
        pub struct HashOutput(pub Sha256Output);

        /// SHA-256 HKDF Output.
        pub struct Sha256HkdfOutput(pub Vec<SecretBufferHandle>);

        /// HKDF Output.
        pub struct HkdfOutput(pub Sha256HkdfOutput);

        /// Handle to an AES-256 Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct Aes256GcmSecretKeyHandle(pub HandleToSecret);

        /// Handle to a AEAD Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct AeadSecretKeyHandle(pub Aes256GcmSecretKeyHandle);
    } else if #[cfg(feature = "OCKAM_XX_25519_AES128_GCM_SHA256")] {
        /// Hash used for Noise handshake.
        pub struct HashOutput(pub Sha256Output);

        /// SHA-256 HKDF Output.
        pub struct Sha256HkdfOutput(pub Vec<SecretBufferHandle>);

        /// HKDF Output.
        pub struct HkdfOutput(pub Sha256HkdfOutput);

        /// Handle to an AES-128 Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct Aes128GcmSecretKeyHandle(pub HandleToSecret);

        /// Handle to a AEAD Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct AeadSecretKeyHandle(pub Aes128GcmSecretKeyHandle);
    } else if #[cfg(feature = "OCKAM_XX_25519_ChaChaPolyBLAKE2s")] {
        /// Blake2s digest length
        pub const BLAKE2S_LENGTH: usize = 32;

        /// Blake2s Output.
        pub struct Blake2sOutput(pub [u8; BLAKE2S_LENGTH]);

        /// Hash used for Noise handshake.
        pub struct HashOutput(pub Blake2sOutput);

        /// Blake2s HKDF Output.
        pub struct Blake2sHkdfOutput(Vec<SecretBufferHandle>);

        /// HKDF Output.
        pub struct HkdfOutput(pub Blake2sHkdfOutput);

        /// Handle to a ChaCha20-Poly1305 Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct Chacha20Poly1305SecretKeyHandle(pub HandleToSecret);

        /// Handle to a AEAD Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
        pub struct AeadSecretKeyHandle(pub Chacha20Poly1305SecretKeyHandle);
    }
}
