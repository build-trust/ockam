use minicbor::Decode;
use ockam_core::Result;

use crate::models::{
    Change, ChangeData, CredentialData, PurposeKeyAttestation, PurposeKeyAttestationData,
    VersionedData,
};
use crate::Credential;

fn get_versioned_data<'a, T: Decode<'a, ()>>(data: &'a [u8]) -> Result<T> {
    Ok(minicbor::decode(data)?)
}

impl Credential {
    /// Extract [`VersionedData`]
    pub fn get_versioned_data(&self) -> Result<VersionedData> {
        get_versioned_data(&self.data)
    }
}

impl PurposeKeyAttestation {
    /// Extract [`VersionedData`]
    pub fn get_versioned_data(&self) -> Result<VersionedData> {
        get_versioned_data(&self.data)
    }
}

impl Change {
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

impl PurposeKeyAttestationData {
    /// Extract [`PurposeKeyAttestationData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        Ok(minicbor::decode(&versioned_data.data)?)
    }
}

impl ChangeData {
    /// Extract [`ChangeData`] from [`VersionedData`]
    pub fn get_data(versioned_data: &VersionedData) -> Result<Self> {
        Ok(minicbor::decode(&versioned_data.data)?)
    }
}
