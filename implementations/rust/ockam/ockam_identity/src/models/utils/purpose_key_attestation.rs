use crate::models::utils::get_versioned_data;
use crate::models::{
    CredentialVerifyingKey, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposeKeyAttestationSignature, VersionedData,
};

use ockam_core::Result;
use ockam_vault::{Signature, VerifyingPublicKey};

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
            PurposeKeyAttestationSignature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            PurposeKeyAttestationSignature::ECDSASHA256CurveP256(value) => {
                Self::ECDSASHA256CurveP256(value)
            }
        }
    }
}

impl From<Signature> for PurposeKeyAttestationSignature {
    fn from(value: Signature) -> Self {
        match value {
            Signature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            Signature::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}

impl From<CredentialVerifyingKey> for VerifyingPublicKey {
    fn from(value: CredentialVerifyingKey) -> Self {
        match value {
            CredentialVerifyingKey::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            CredentialVerifyingKey::ECDSASHA256CurveP256(value) => {
                Self::ECDSASHA256CurveP256(value)
            }
        }
    }
}

impl From<VerifyingPublicKey> for CredentialVerifyingKey {
    fn from(value: VerifyingPublicKey) -> Self {
        match value {
            VerifyingPublicKey::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}
