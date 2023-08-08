use super::super::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use super::super::Purpose;
use ockam_vault::{KeyId, SecretType};

/// Identity implementation
#[derive(Clone, Debug)]
pub struct PurposeKey {
    subject: Identifier,
    key_id: KeyId,
    stype: SecretType,
    purpose: Purpose,
    data: PurposeKeyAttestationData,
    attestation: PurposeKeyAttestation,
}

impl PurposeKey {
    pub fn new(
        subject: Identifier,
        key_id: KeyId,
        stype: SecretType,
        purpose: Purpose,
        data: PurposeKeyAttestationData,
        attestation: PurposeKeyAttestation,
    ) -> Self {
        Self {
            subject,
            key_id,
            stype,
            purpose,
            data,
            attestation,
        }
    }
    pub fn subject(&self) -> &Identifier {
        &self.subject
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
    pub fn data(&self) -> &PurposeKeyAttestationData {
        &self.data
    }
}
