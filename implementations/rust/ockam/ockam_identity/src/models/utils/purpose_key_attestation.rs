use crate::models::{
    CredentialVerifyingKey, PurposeKeyAttestation, PurposeKeyAttestationData,
    PurposeKeyAttestationSignature, VersionedData, PURPOSE_KEY_ATTESTATION_DATA_TYPE,
};
use crate::IdentityError;

use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{Signature, VerifyingPublicKey};

impl PurposeKeyAttestation {
    /// Create [`VersionedData`] with corresponding version and data_type
    pub fn create_versioned_data(data: Vec<u8>) -> VersionedData {
        VersionedData {
            version: 1,
            data_type: PURPOSE_KEY_ATTESTATION_DATA_TYPE,
            data,
        }
    }
}

impl PurposeKeyAttestationData {
    /// Extract [`PurposeKeyAttestationData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownPurposeKeyAttestationVersion)?;
        }

        if versioned_data.data_type != PURPOSE_KEY_ATTESTATION_DATA_TYPE {
            return Err(IdentityError::InvalidPurposeKeyAttestationDataType)?;
        }

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
