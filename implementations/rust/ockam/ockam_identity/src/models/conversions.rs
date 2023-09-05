use crate::models::{
    ChangeSignature, CredentialSignature, CredentialSigningKey, Ed25519PublicKey, Ed25519Signature,
    P256ECDSAPublicKey, P256ECDSASignature, PrimaryPublicKey, PurposeKeyAttestationSignature,
    X25519PublicKey,
};
use crate::{IdentityError, TimestampInSeconds};

use ockam_core::{Error, Result};
use ockam_vault::{PublicKey, SecretType, Signature};

impl From<PrimaryPublicKey> for PublicKey {
    fn from(value: PrimaryPublicKey) -> Self {
        match value {
            PrimaryPublicKey::Ed25519PublicKey(value) => Self::from(value),
            PrimaryPublicKey::P256ECDSAPublicKey(value) => Self::from(value),
        }
    }
}

impl TryFrom<PublicKey> for PrimaryPublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::Ed25519 => Ok(Self::Ed25519PublicKey(Ed25519PublicKey(
                value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?,
            ))),
            SecretType::NistP256 => Ok(Self::P256ECDSAPublicKey(P256ECDSAPublicKey(
                value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?,
            ))),

            SecretType::X25519 | SecretType::Buffer | SecretType::Aes => {
                Err(IdentityError::InvalidKeyType.into())
            }
        }
    }
}

impl From<Ed25519Signature> for Signature {
    fn from(value: Ed25519Signature) -> Self {
        Self::new(value.0.to_vec())
    }
}

impl From<P256ECDSASignature> for Signature {
    fn from(value: P256ECDSASignature) -> Self {
        Self::new(value.0.to_vec())
    }
}

impl From<CredentialSignature> for Signature {
    fn from(value: CredentialSignature) -> Self {
        match value {
            CredentialSignature::Ed25519Signature(value) => Self::new(value.0.to_vec()),
            CredentialSignature::P256ECDSASignature(value) => Self::new(value.0.to_vec()),
        }
    }
}

impl CredentialSignature {
    /// Try to create a [`CredentialSignature`] using a binary [`Signature`] and its type
    pub fn try_from_signature(signature: Signature, stype: SecretType) -> Result<Self> {
        match stype {
            SecretType::Ed25519 => Ok(Self::Ed25519Signature(Ed25519Signature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),
            SecretType::NistP256 => Ok(Self::P256ECDSASignature(P256ECDSASignature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),

            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                Err(IdentityError::InvalidKeyType.into())
            }
        }
    }
}

impl From<PurposeKeyAttestationSignature> for Signature {
    fn from(value: PurposeKeyAttestationSignature) -> Self {
        match value {
            PurposeKeyAttestationSignature::Ed25519Signature(value) => Self::new(value.0.to_vec()),
            PurposeKeyAttestationSignature::P256ECDSASignature(value) => {
                Self::new(value.0.to_vec())
            }
        }
    }
}

impl PurposeKeyAttestationSignature {
    /// Try to create a [`PurposeKeyAttestationSignature`] using a binary [`Signature`] and its type
    pub fn try_from_signature(signature: Signature, stype: SecretType) -> Result<Self> {
        match stype {
            SecretType::Ed25519 => Ok(Self::Ed25519Signature(Ed25519Signature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),
            SecretType::NistP256 => Ok(Self::P256ECDSASignature(P256ECDSASignature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),

            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                Err(IdentityError::InvalidKeyType.into())
            }
        }
    }
}

impl From<ChangeSignature> for Signature {
    fn from(value: ChangeSignature) -> Self {
        match value {
            ChangeSignature::Ed25519Signature(value) => Self::new(value.0.to_vec()),
            ChangeSignature::P256ECDSASignature(value) => Self::new(value.0.to_vec()),
        }
    }
}

impl ChangeSignature {
    /// Try to create a [`ChangeSignature`] using a binary [`Signature`] and its type
    pub fn try_from_signature(signature: Signature, stype: SecretType) -> Result<Self> {
        match stype {
            SecretType::Ed25519 => Ok(Self::Ed25519Signature(Ed25519Signature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),
            SecretType::NistP256 => Ok(Self::P256ECDSASignature(P256ECDSASignature(
                signature
                    .as_ref()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidSignatureData)?,
            ))),

            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                Err(IdentityError::InvalidKeyType.into())
            }
        }
    }
}

impl From<Ed25519PublicKey> for PublicKey {
    fn from(value: Ed25519PublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::Ed25519)
    }
}

impl From<X25519PublicKey> for PublicKey {
    fn from(value: X25519PublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::X25519)
    }
}

impl From<P256ECDSAPublicKey> for PublicKey {
    fn from(value: P256ECDSAPublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::NistP256)
    }
}

impl TryFrom<PublicKey> for Ed25519PublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::Ed25519 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl TryFrom<PublicKey> for X25519PublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::X25519 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl TryFrom<PublicKey> for P256ECDSAPublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::NistP256 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl From<CredentialSigningKey> for PublicKey {
    fn from(value: CredentialSigningKey) -> Self {
        match value {
            CredentialSigningKey::Ed25519PublicKey(key) => key.into(),
            CredentialSigningKey::P256ECDSAPublicKey(key) => key.into(),
        }
    }
}

impl TryFrom<PublicKey> for CredentialSigningKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::Ed25519 => Ok(Self::Ed25519PublicKey(value.try_into()?)),
            SecretType::NistP256 => Ok(Self::P256ECDSAPublicKey(value.try_into()?)),

            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl core::ops::Deref for TimestampInSeconds {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u64> for TimestampInSeconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl core::ops::Add<TimestampInSeconds> for TimestampInSeconds {
    type Output = TimestampInSeconds;

    fn add(self, rhs: TimestampInSeconds) -> Self::Output {
        TimestampInSeconds(self.0 + rhs.0)
    }
}
