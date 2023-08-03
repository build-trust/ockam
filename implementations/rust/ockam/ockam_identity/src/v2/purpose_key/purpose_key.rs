use super::super::models::PurposeKeyAttestation;
use super::super::Purpose;
use ockam_vault::{KeyId, SecretType};

/// Identity implementation
#[derive(Clone, Debug)]
pub struct PurposeKey {
    key_id: KeyId,
    stype: SecretType,
    purpose: Purpose,
    attestation: PurposeKeyAttestation,
}

impl PurposeKey {
    pub fn new(
        key_id: KeyId,
        stype: SecretType,
        purpose: Purpose,
        attestation: PurposeKeyAttestation,
    ) -> Self {
        Self {
            key_id,
            stype,
            purpose,
            attestation,
        }
    }
    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }
    pub fn attestation(&self) -> &PurposeKeyAttestation {
        &self.attestation
    }
    pub fn stype(&self) -> SecretType {
        self.stype
    }
}
