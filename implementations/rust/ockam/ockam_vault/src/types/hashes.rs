use crate::{HandleToSecret, SecretBufferHandle};
use cfg_if::cfg_if;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::vec::Vec;

/// SHA256 digest length
pub const SHA256_LENGTH: usize = 32;

/// SHA-256 Output.
#[derive(Encode, Decode, CborLen)]
pub struct Sha256Output(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; SHA256_LENGTH]);

/// Handle to an AES-256 Secret Key.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
pub struct AeadSecretKeyHandle(#[n(0)] pub AeadSecretKeyHandleType);

impl AeadSecretKeyHandle {
    /// Constructor
    pub fn new(handle: HandleToSecret) -> Self {
        Self(AeadSecretKeyHandleType::new(handle))
    }
}

cfg_if! {
    if #[cfg(any(not(feature = "disable_default_noise_protocol"), feature = "OCKAM_XX_25519_AES256_GCM_SHA256"))] {
        /// Hash used for Noise handshake.
        #[derive(Encode, Decode, CborLen)]
        pub struct HashOutput(#[n(0)] pub Sha256Output);

        /// SHA-256 HKDF Output.
        #[derive(Encode, Decode, CborLen)]
        pub struct Sha256HkdfOutput(#[n(0)] pub Vec<SecretBufferHandle>);

        /// HKDF Output.
        #[derive(Encode, Decode, CborLen)]
        pub struct HkdfOutput(#[n(0)] pub Sha256HkdfOutput);

        /// Handle to an AES-256 Secret Key.
        #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
        pub struct Aes256GcmSecretKeyHandle(#[n(0)] pub HandleToSecret);

        impl Aes256GcmSecretKeyHandle {
            /// Constructor
            pub fn new(handle: HandleToSecret) -> Self {
                Self(handle)
            }
        }

        /// Handle to a AEAD Secret Key.
        pub type AeadSecretKeyHandleType = Aes256GcmSecretKeyHandle;
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

        impl Aes128GcmSecretKeyHandle {
            /// Constructor
            pub fn new(handle: HandleToSecret) -> Self {
                Self(handle)
            }
        }

        /// Handle to a AEAD Secret Key.
        pub type AeadSecretKeyHandleType = Aes128GcmSecretKeyHandle;
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

        impl Chacha20Poly1305SecretKeyHandle {
            /// Constructor
            pub fn new(handle: HandleToSecret) -> Self {
                Self(handle)
            }
        }

        /// Handle to a AEAD Secret Key.
        pub type AeadSecretKeyHandleType = Chacha20Poly1305SecretKeyHandle;
    }
}
