use crate::{error::*, CredentialPresentation, PresentationManifest};
use bbs::prelude::{DeterministicPublicKey, HashElem, PoKOfSignatureProofStatus, ProofChallenge};
use digest::{generic_array::GenericArray, Digest, FixedOutput};
use ockam_core::lib::*;

/// Methods for verifying presentations
#[derive(Debug)]
pub struct Verifier;

impl Verifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    pub fn create_proof_request_id() -> [u8; 32] {
        bbs::verifier::Verifier::generate_proof_nonce().to_bytes_compressed_form()
    }

    /// Check if the credential presentations are valid
    pub fn verify_credential_presentations(
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: [u8; 32],
        challenge: [u8; 32],
    ) -> Result<(), CredentialError> {
        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();

        let mut vks = Vec::new();
        for i in 0..presentations.len() {
            let prez = &presentations[i];
            let pm = &presentation_manifests[i];
            let vk = DeterministicPublicKey::from(pm.public_key)
                .to_public_key(presentation_manifests[i].credential_schema.attributes.len())
                .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;

            let mut hasher = sha2::Sha256::new();
            hasher.input(&bytes);
            hasher.input(
                prez.proof
                    .get_bytes_for_challenge(pm.revealed.iter().map(|i| *i).collect(), &vk),
            );
            bytes = hasher.result();
            vks.push(vk);
        }

        let mut hasher = sha2::Sha256::new();
        hasher.input(&bytes);
        hasher.input(&proof_request_id);
        let challenge_verifier = ProofChallenge::hash(&hasher.result());
        let challenge = ProofChallenge::from(challenge);

        if challenge != challenge_verifier {
            return Err(CredentialError::InvalidPresentationChallenge);
        }

        for i in 0..vks.len() {
            let vk = &vks[i];
            let pm = &presentation_manifests[i];
            let proof = &presentations[i];

            let msgs = pm
                .revealed
                .iter()
                .zip(proof.revealed_attributes.iter())
                .map(|(i, a)| (*i, a.to_signature_message()))
                .collect();
            match proof.proof.verify(&vk, &msgs, &challenge_verifier) {
                Ok(status) => {
                    if !matches!(status, PoKOfSignatureProofStatus::Success) {
                        return Err(CredentialError::InvalidCredentialPresentation(
                            (i + 1) as u32,
                        ));
                    }
                }
                Err(_) => {
                    return Err(CredentialError::InvalidCredentialPresentation(
                        (i + 1) as u32,
                    ))
                }
            };
        }
        Ok(())
    }
}
