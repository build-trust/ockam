use crate::models::utils::get_versioned_data;
use crate::models::{
    CredentialData, CredentialSignature, Ed25519Signature, P256ECDSASignature, VersionedData,
};
use crate::{Credential, IdentityError};

use ockam_core::Result;
use ockam_vault::{SecretType, Signature};

impl Credential {
    /// Extract [`VersionedData`]
    pub fn get_versioned_data(&self) -> Result<VersionedData> {
        get_versioned_data(&self.data)
    }
}

impl CredentialData {
    /// Extract [`CredentialData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        Ok(minicbor::decode(&versioned_data.data)?)
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
