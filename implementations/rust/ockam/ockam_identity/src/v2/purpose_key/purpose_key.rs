use super::super::models::PurposeKeyAttestation;
use super::super::Purpose;
use ockam_vault::KeyId;

/// Identity implementation
#[derive(Clone, Debug)]
pub struct PurposeKey {
    key_id: KeyId,
    purpose: Purpose,
    attestation: PurposeKeyAttestation,
}

impl PurposeKey {
    pub fn new(key_id: KeyId, purpose: Purpose, attestation: PurposeKeyAttestation) -> Self {
        Self {
            key_id,
            purpose,
            attestation,
        }
    }
}
