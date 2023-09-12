use ockam_vault::{KeyId, PublicKey, SecretType};

use crate::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use crate::Purpose;

/// Own PurposeKey
#[derive(Clone, Debug)]
pub struct PurposeKey {
    subject: Identifier,
    key_id: KeyId,
    public_key: PublicKey,
    purpose: Purpose,
    data: PurposeKeyAttestationData,
    attestation: PurposeKeyAttestation,
}

impl PurposeKey {
    /// Constructor
    pub fn new(
        subject: Identifier,
        key_id: KeyId,
        public_key: PublicKey,
        purpose: Purpose,
        data: PurposeKeyAttestationData,
        attestation: PurposeKeyAttestation,
    ) -> Self {
        Self {
            subject,
            key_id,
            public_key,
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
    /// Public Key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    /// Secret Type
    pub fn stype(&self) -> SecretType {
        self.public_key.stype()
    }
    /// Purpose of the Purpose Key
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }
    /// Attestation proving that Purpose Key is owned by the Subject
    pub fn attestation(&self) -> &PurposeKeyAttestation {
        &self.attestation
    }
    /// Data inside [`PurposeKeyAttestation`]
    pub fn data(&self) -> &PurposeKeyAttestationData {
        &self.data
    }
}
