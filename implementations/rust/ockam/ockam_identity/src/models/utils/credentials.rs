use crate::models::utils::get_versioned_data;
use crate::models::{CredentialData, CredentialSignature, VersionedData};
use crate::Credential;

use ockam_core::Result;
use ockam_vault::Signature;

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
            CredentialSignature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            CredentialSignature::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}

impl From<Signature> for CredentialSignature {
    fn from(value: Signature) -> Self {
        match value {
            Signature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            Signature::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}
