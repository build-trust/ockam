use crate::{VaultError, EDDSA_CURVE25519_PUBLIC_KEY_LENGTH, EDDSA_CURVE25519_SIGNATURE_LENGTH};
use ockam_core::Result;
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

impl SigningSecret {
    /// Return the secret key
    pub fn key(&self) -> &[u8; 32] {
        match self {
            SigningSecret::EdDSACurve25519(k) => k.key(),
            SigningSecret::ECDSASHA256CurveP256(k) => k.key(),
        }
    }

    /// Return the u8 representation of this signing secret type
    pub fn type_as_u8(&self) -> u8 {
        match self {
            SigningSecret::EdDSACurve25519(_) => 0,
            SigningSecret::ECDSASHA256CurveP256(_) => 1,
        }
    }

    /// Create a new `SigningSecret` from a 32-byte key.
    pub fn from_key(key: &[u8], key_type: u8) -> Result<Self> {
        match key_type {
            0 => {
                let k: [u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH] =
                    key.try_into().map_err(|_| VaultError::InvalidKeyType)?;
                Ok(SigningSecret::EdDSACurve25519(
                    EdDSACurve25519SecretKey::new(k),
                ))
            }
            1 => {
                let k: [u8; ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH] =
                    key.try_into().map_err(|_| VaultError::InvalidKeyType)?;
                Ok(SigningSecret::ECDSASHA256CurveP256(
                    ECDSASHA256CurveP256SecretKey::new(k),
                ))
            }
            _ => Err(VaultError::InvalidKeyType.into()),
        }
    }
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
