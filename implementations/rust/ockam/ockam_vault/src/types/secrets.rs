use core::fmt::Debug;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::vec::Vec;

/// Implementation-specific arbitrary vector of bytes that allows a concrete Vault implementation
/// to address a specific secret that it stores.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct HandleToSecret(#[cbor(n(0), with = "minicbor::bytes")]  Vec<u8>);

impl Debug for HandleToSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "HandleToSecret({})", crate::types::debug_hash(&self.0))
    }
}

impl HandleToSecret {
    /// Constructor.
    pub fn new(value: Vec<u8>) -> Self {
        Self(value)
    }

    /// Get value.
    pub fn value(&self) -> &Vec<u8> {
        &self.0
    }

    /// Take value.
    pub fn take_value(self) -> Vec<u8> {
        self.0
    }
}

/// A handle to signing secret key inside a vault.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum SigningSecretKeyHandle {
    /// Curve25519 key that is only used for EdDSA signatures.
    #[n(0)] EdDSACurve25519(#[n(0)] HandleToSecret),
    /// Curve P-256 key that is only used for ECDSA SHA256 signatures.
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] HandleToSecret),
}

impl SigningSecretKeyHandle {
    /// [`HandleToSecret`]
    pub fn handle(&self) -> &HandleToSecret {
        match self {
            SigningSecretKeyHandle::EdDSACurve25519(handle) => handle,
            SigningSecretKeyHandle::ECDSASHA256CurveP256(handle) => handle,
        }
    }
}

/// Key type for Signing. See [`super::signatures::Signature`].
#[derive(Debug, Eq, PartialEq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum SigningKeyType {
    /// See [`super::signatures::EdDSACurve25519Signature`]
    #[n(0)] EdDSACurve25519,
    /// See [`super::signatures::ECDSASHA256CurveP256Signature`]
    #[n(1)] ECDSASHA256CurveP256,
}

/// A handle to a X25519 Secret Key.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
pub struct X25519SecretKeyHandle(#[n(0)] pub HandleToSecret);

/// A handle to a secret Buffer (like an HKDF output).
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
pub struct SecretBufferHandle(#[n(0)] pub HandleToSecret);
