use crate::models::{CredentialData, CredentialSignature, VersionedData, CREDENTIAL_DATA_TYPE};
use crate::{Credential, IdentityError};

use ockam_core::compat::str;
use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::Signature;

impl Credential {
    /// Create [`VersionedData`] with corresponding version and data_type
    pub fn create_versioned_data(data: Vec<u8>) -> VersionedData {
        VersionedData {
            version: 1,
            data_type: CREDENTIAL_DATA_TYPE,
            data,
        }
    }

    /// Extract [`CredentialData`]
    pub fn get_credential_data(&self) -> Result<CredentialData> {
        CredentialData::get_data(&minicbor::decode(&self.data)?)
    }
}

impl CredentialData {
    /// Extract [`CredentialData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        if versioned_data.version != 1 {
            return Err(IdentityError::UnknownCredentialVersion)?;
        }

        if versioned_data.data_type != CREDENTIAL_DATA_TYPE {
            return Err(IdentityError::InvalidCredentialDataType)?;
        }

        Ok(minicbor::decode(&versioned_data.data)?)
    }

    /// Return the credential's attributes as a displayable string
    pub fn get_attributes_display(&self) -> String {
        self.subject_attributes
            .map
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    str::from_utf8(k).unwrap_or("**binary**"),
                    str::from_utf8(v).unwrap_or("**binary**")
                )
            })
            .collect::<Vec<String>>()
            .join(";")
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
