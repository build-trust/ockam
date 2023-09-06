use crate::models::utils::get_versioned_data;
use crate::models::{
    CredentialSigningKey, Ed25519Signature, P256ECDSASignature, PurposeKeyAttestation,
    PurposeKeyAttestationData, PurposeKeyAttestationSignature, VersionedData,
};
use crate::IdentityError;

use ockam_core::{Error, Result};
use ockam_vault::{PublicKey, SecretType, Signature};

impl PurposeKeyAttestation {
    /// Extract [`VersionedData`]
    pub fn get_versioned_data(&self) -> Result<VersionedData> {
        get_versioned_data(&self.data)
    }
}

impl PurposeKeyAttestationData {
    /// Extract [`PurposeKeyAttestationData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        Ok(minicbor::decode(&versioned_data.data)?)
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
