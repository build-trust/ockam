use crate::models::{
    Change, ChangeData, ChangeHistory, ChangeSignature, PrimaryPublicKey, VersionedData,
    CHANGE_DATA_TYPE,
};

use crate::IdentityError;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::{Signature, VerifyingPublicKey};

impl Change {
    /// Create [`VersionedData`] with corresponding version and data_type
    pub fn create_versioned_data(data: Vec<u8>) -> VersionedData {
        VersionedData {
            version: 1,
            data_type: CHANGE_DATA_TYPE,
            data,
        }
    }
}

impl ChangeData {
    /// Extract [`ChangeData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownIdentityVersion.into());
        }

        if versioned_data.data_type != CHANGE_DATA_TYPE {
            return Err(IdentityError::InvalidIdentityDataType.into());
        }

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

impl From<PrimaryPublicKey> for VerifyingPublicKey {
    fn from(value: PrimaryPublicKey) -> Self {
        match value {
            PrimaryPublicKey::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            PrimaryPublicKey::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}

impl From<VerifyingPublicKey> for PrimaryPublicKey {
    fn from(value: VerifyingPublicKey) -> Self {
        match value {
            VerifyingPublicKey::EdDSACurve25519(value) => PrimaryPublicKey::EdDSACurve25519(value),
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => {
                PrimaryPublicKey::ECDSASHA256CurveP256(value)
            }
        }
    }
}

impl From<ChangeSignature> for Signature {
    fn from(value: ChangeSignature) -> Self {
        match value {
            ChangeSignature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            ChangeSignature::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}

impl From<Signature> for ChangeSignature {
    fn from(value: Signature) -> Self {
        match value {
            Signature::EdDSACurve25519(value) => Self::EdDSACurve25519(value),
            Signature::ECDSASHA256CurveP256(value) => Self::ECDSASHA256CurveP256(value),
        }
    }
}
