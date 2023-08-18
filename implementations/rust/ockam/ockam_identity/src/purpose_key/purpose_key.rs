use super::super::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use super::super::Purpose;
use ockam_vault::{KeyId, SecretType};

/// Own PurposeKey
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
    /// Constructor
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
    /// Owner of the Purpose Key
    pub fn subject(&self) -> &Identifier {
        &self.subject
    }
    /// Key id of the corresponding Private key
    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }
    /// Purpose of the Purpose Key
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }
    /// Attestation proving that Purpose Key is owned by the Subject
    pub fn attestation(&self) -> &PurposeKeyAttestation {
        &self.attestation
    }
    /// Secret Type
    pub fn stype(&self) -> SecretType {
        self.stype
    }
    /// Data inside [`PurposeKeyAttestation`]
    pub fn data(&self) -> &PurposeKeyAttestationData {
        &self.data
    }
}
