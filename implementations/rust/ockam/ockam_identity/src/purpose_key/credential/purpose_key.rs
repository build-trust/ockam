use crate::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use ockam_vault::{SigningSecretKeyHandle, VerifyingPublicKey};

/// Own PurposeKey
#[derive(Clone, Debug)]
pub struct CredentialPurposeKey {
    subject: Identifier,
    key: SigningSecretKeyHandle,
    public_key: VerifyingPublicKey,
    data: PurposeKeyAttestationData,
    attestation: PurposeKeyAttestation,
}

impl CredentialPurposeKey {
    /// Constructor
    pub fn new(
        subject: Identifier,
        key: SigningSecretKeyHandle,
        public_key: VerifyingPublicKey,
        data: PurposeKeyAttestationData,
        attestation: PurposeKeyAttestation,
    ) -> Self {
        Self {
            subject,
            key,
            public_key,
            data,
            attestation,
        }
    }
    /// Owner of the Purpose Key
    pub fn subject(&self) -> &Identifier {
        &self.subject
    }
    /// Key id of the corresponding Private key
    pub fn key(&self) -> &SigningSecretKeyHandle {
        &self.key
    }
    /// Public Key
    pub fn public_key(&self) -> &VerifyingPublicKey {
        &self.public_key
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
