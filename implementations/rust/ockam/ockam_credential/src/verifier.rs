use crate::{error::*, CredentialPresentation, PresentationManifest};
use bbs::prelude::{DeterministicPublicKey, HashElem, PoKOfSignatureProofStatus, ProofChallenge};
use digest::{generic_array::GenericArray, Digest, FixedOutput};
use ff::Field;
use ockam_core::lib::*;
use pairing_plus::{
    bls12_381::{Bls12, Fq12, G1, G2},
    hash_to_curve::HashToCurve,
    hash_to_field::ExpandMsgXmd,
    serdes::SerDes,
    CurveAffine, CurveProjective, Engine,
};

/// Methods for verifying presentations
#[derive(Debug)]
pub struct Verifier;

impl Verifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    pub fn create_proof_request_id() -> [u8; 32] {
        bbs::verifier::Verifier::generate_proof_nonce().to_bytes_compressed_form()
    }

    /// Verify a proof of possession
    pub fn verify_proof_of_possession(issuer_vk: [u8; 96], proof: [u8; 48]) -> bool {
        let p = <G1 as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(
            &issuer_vk,
            crate::issuer::CSUITE_POP,
        )
        .into_affine()
        .prepare();
        let g2 = {
            let mut t = G2::one();
            t.negate();
            t.into_affine().prepare()
        };
        let mut c = std::io::Cursor::new(issuer_vk);
        if let Ok(pk) = G2::deserialize(&mut c, true) {
            let mut c = std::io::Cursor::new(proof);
            if let Ok(sig) = G1::deserialize(&mut c, true) {
                return match Bls12::final_exponentiation(&Bls12::miller_loop(&[
                    (&p, &pk.into_affine().prepare()),
                    (&sig.into_affine().prepare(), &g2),
                ])) {
                    None => false,
                    Some(pp) => pp == Fq12::one(),
                };
            }
        }
        false
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
        let challenge = ProofChallenge::from(presentations[0].presentation_id);

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
