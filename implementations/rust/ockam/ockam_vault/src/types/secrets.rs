use ockam_core::compat::vec::Vec;

/// A handle to a secret inside a vault.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct HandleToSecret(Vec<u8>);

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
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SigningSecretKeyHandle {
    /// Curve25519 key that is only used for EdDSA signatures.
    EdDSACurve25519(HandleToSecret),
    /// Curve P-256 key that is only used for ECDSA SHA256 signatures.
    ECDSASHA256CurveP256(HandleToSecret),
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
#[derive(Debug, Eq, PartialEq)]
pub enum SigningKeyType {
    /// See [`super::signatures::EdDSACurve25519Signature`]
    EdDSACurve25519,
    /// See [`super::signatures::ECDSASHA256CurveP256Signature`]
    ECDSASHA256CurveP256,
}

/// A handle to a X25519 Secret Key.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct X25519SecretKeyHandle(pub HandleToSecret);

/// A handle to a secret Buffer (like an HKDF output).
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct SecretBufferHandle(pub HandleToSecret);
