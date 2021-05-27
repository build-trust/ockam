use crate::*;
use rand::RngCore;
use sha2::digest::{generic_array::GenericArray, Digest, FixedOutput};
use signature_bbs_plus::{MessageGenerators, PublicKey};
use signature_bls::ProofOfPossession;
use signature_core::lib::{Message, *};

use ockam_core::lib::Result;

/// Methods for verifying presentations
#[derive(Debug)]
pub struct CredentialVerifier;

impl CredentialVerifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    pub fn create_proof_request_id(rng: impl RngCore) -> [u8; 32] {
        Nonce::random(rng).to_bytes()
    }

    /// Verify a proof of possession
    pub fn verify_proof_of_possession(issuer_vk: [u8; 96], proof: [u8; 48]) -> bool {
        let vk = PublicKey::from_bytes(&issuer_vk);
        let proof = ProofOfPossession::from_bytes(&proof);

        if vk.is_some().unwrap_u8() == 1 && proof.is_some().unwrap_u8() == 1 {
            let pk = vk.unwrap();
            let p = proof.unwrap();
            p.verify(pk).unwrap_u8() == 1
        } else {
            false
        }
    }

    /// Check if the credential presentations are valid
    pub fn verify_credential_presentations(
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: [u8; 32],
    ) -> Result<(), CredentialError> {
        if presentations.len() != presentation_manifests.len() || presentations.len() == 0 {
            return Err(CredentialError::MismatchedPresentationAndManifests);
        }

        if presentations
            .iter()
            .any(|p| p.presentation_id != presentations[0].presentation_id)
        {
            return Err(CredentialError::MismatchedPresentationAndManifests);
        }

        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();
        let challenge = Challenge::from_bytes(&presentations[0].presentation_id).unwrap();

        for i in 0..presentations.len() {
            let prez = &presentations[i];
            let pm = &presentation_manifests[i];
            let vk = PublicKey::from_bytes(&pm.public_key).unwrap();

            if !prez.proof.verify(vk) {
                return Err(CredentialError::InvalidCredentialPresentation(i as u32));
            }

            let generators =
                MessageGenerators::from_public_key(vk, pm.credential_schema.attributes.len());
            let msgs = pm
                .revealed
                .iter()
                .zip(prez.revealed_attributes.iter())
                .map(|(i, r)| (*i, r.to_signature_message()))
                .collect::<Vec<(usize, Message), 64>>();

            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);
            prez.proof
                .add_challenge_contribution(&generators, &msgs, challenge, &mut hasher);
            bytes = hasher.finalize();
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        hasher.update(&proof_request_id);
        let challenge_verifier = Challenge::hash(&hasher.finalize());

        if challenge != challenge_verifier {
            return Err(CredentialError::InvalidPresentationChallenge);
        }

        Ok(())
    }
}
