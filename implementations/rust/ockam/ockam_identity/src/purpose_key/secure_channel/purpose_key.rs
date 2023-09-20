use crate::models::{Identifier, PurposeKeyAttestation, PurposeKeyAttestationData};
use ockam_vault::{X25519PublicKey, X25519SecretKeyHandle};

/// Own PurposeKey
#[derive(Clone, Debug)]
pub struct SecureChannelPurposeKey {
    subject: Identifier,
    key: X25519SecretKeyHandle,
    public_key: X25519PublicKey,
    data: PurposeKeyAttestationData,
    attestation: PurposeKeyAttestation,
}

impl SecureChannelPurposeKey {
    /// Constructor
    pub fn new(
        subject: Identifier,
        key: X25519SecretKeyHandle,
        public_key: X25519PublicKey,
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
    pub fn key(&self) -> &X25519SecretKeyHandle {
        &self.key
    }
    /// Public Key
    pub fn public_key(&self) -> &X25519PublicKey {
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
