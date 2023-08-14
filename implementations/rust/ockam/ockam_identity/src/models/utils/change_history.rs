use crate::models::utils::get_versioned_data;
use crate::models::{
    Change, ChangeData, ChangeHistory, ChangeSignature, Ed25519PublicKey, Ed25519Signature,
    P256ECDSAPublicKey, P256ECDSASignature, PrimaryPublicKey, VersionedData,
};
use crate::IdentityError;

use ockam_core::compat::vec::Vec;
use ockam_core::{Error, Result};
use ockam_vault::{PublicKey, SecretType, Signature};

impl Change {
    /// Extract [`VersionedData`]
    pub fn get_versioned_data(&self) -> Result<VersionedData> {
        get_versioned_data(&self.data)
    }
}

impl ChangeData {
    /// Extract [`ChangeData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        Ok(minicbor::decode(&versioned_data.data)?)
    }
}

impl ChangeHistory {
    /// Export [`ChangeHistory`] to a binary format using CBOR
    pub fn export(&self) -> Result<Vec<u8>> {
        Ok(minicbor::to_vec(self)?)
    }

    /// Import [`ChangeHistory`] from a binary format using CBOR
    pub fn import(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

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
