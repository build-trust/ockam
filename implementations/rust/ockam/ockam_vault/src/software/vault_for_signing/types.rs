use crate::{EDDSA_CURVE25519_PUBLIC_KEY_LENGTH, EDDSA_CURVE25519_SIGNATURE_LENGTH};
use static_assertions::const_assert_eq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Ed25519 private key length.
pub const EDDSA_CURVE25519_SECRET_KEY_LENGTH: usize = 32;

/// NIST P256 private key length.
pub const ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH: usize = 32;

/// EdDSACurve25519 Secret Key.
#[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct EdDSACurve25519SecretKey([u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH]);

impl EdDSACurve25519SecretKey {
    /// Constructor.
    pub fn new(key: [u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH]) -> Self {
        Self(key)
    }

    pub(crate) fn key(&self) -> &[u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH] {
        &self.0
    }
}

/// ECDSASHA256CurveP256 Secret Key.
#[derive(Eq, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct ECDSASHA256CurveP256SecretKey([u8; ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH]);

impl ECDSASHA256CurveP256SecretKey {
    /// Constructor.
    pub fn new(key: [u8; ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH]) -> Self {
        Self(key)
    }

    pub(crate) fn key(&self) -> &[u8; ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH] {
        &self.0
    }
}

/// Signing secret binary
#[derive(Eq, PartialEq, Clone, Zeroize)]
pub enum SigningSecret {
    /// Curve25519 key that is only used for EdDSA signatures.
    EdDSACurve25519(EdDSACurve25519SecretKey),
    /// Curve P-256 key that is only used for ECDSA SHA256 signatures.
    ECDSASHA256CurveP256(ECDSASHA256CurveP256SecretKey),
}

const_assert_eq!(
    ed25519_dalek::SECRET_KEY_LENGTH,
    EDDSA_CURVE25519_SECRET_KEY_LENGTH
);

const_assert_eq!(
    ed25519_dalek::PUBLIC_KEY_LENGTH,
    EDDSA_CURVE25519_PUBLIC_KEY_LENGTH
);

const_assert_eq!(
    ed25519_dalek::SIGNATURE_LENGTH,
    EDDSA_CURVE25519_SIGNATURE_LENGTH
);
