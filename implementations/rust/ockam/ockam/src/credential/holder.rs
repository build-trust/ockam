use super::*;
use ockam_core::lib::{HashSet, Vec};
use rand::{CryptoRng, RngCore};
use sha2::digest::{generic_array::GenericArray, Digest, FixedOutput};
use signature_bbs_plus::{MessageGenerators, Prover, PublicKey};
use signature_core::lib::*;

/// The label to indicate the secretid attribute in a schema/credential
pub const SECRET_ID: &str = "secret_id";

/// Represents a holder of a credential
#[derive(Debug)]
pub struct CredentialHolder {
    pub(crate) id: Message,
}

impl CredentialHolder {
    /// Create a new CredentialHolder with a new unique id
    pub fn new(rng: impl RngCore + CryptoRng) -> Self {
        Self {
            id: Message::random(rng),
        }
    }

    /// Accepts a credential offer from an issuer
    pub fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_pk: [u8; 96],
        rng: impl RngCore + CryptoRng,
    ) -> Result<(CredentialRequest, CredentialFragment1), CredentialError> {
        let nonce = Nonce::from_bytes(&offer.id).unwrap();
        let mut i = 0;
        let mut found = false;
        for (j, att) in offer.schema.attributes.iter().enumerate() {
            if att.label == SECRET_ID {
                i = j;
                found = true;
                break;
            }
        }
        if !found {
            return Err(CredentialError::InvalidCredentialSchema);
        }

        let pk = PublicKey::from_bytes(&issuer_pk).unwrap();
        let generators = MessageGenerators::from_public_key(pk, offer.schema.attributes.len());
        let (context, blinding) =
            Prover::new_blind_signature_context(&[(i, self.id)], &generators, nonce, rng)
                .map_err(|_| CredentialError::InvalidCredentialOffer)?;
        Ok((
            CredentialRequest {
                offer_id: offer.id,
                context,
            },
            CredentialFragment1 {
                schema: offer.schema.clone(),
                blinding,
            },
        ))
    }

    /// Combine credential fragments to yield a completed credential
    pub fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Credential {
        let mut attributes = credential_fragment2.attributes;
        for i in 0..credential_fragment1.schema.attributes.len() {
            if credential_fragment1.schema.attributes[i].label == SECRET_ID {
                attributes.insert(i, CredentialAttribute::Blob(self.id.to_bytes()));
                break;
            }
        }
        Credential {
            attributes,
            signature: credential_fragment2
                .signature
                .to_unblinded(credential_fragment1.blinding),
        }
    }

    /// Check a credential to make sure its valid
    pub fn is_valid_credential(&self, credential: &Credential, verkey: [u8; 96]) -> bool {
        // credential cannot have zero attributes so unwrap is okay
        let vk = PublicKey::from_bytes(&verkey).unwrap();
        let generators = MessageGenerators::from_public_key(vk, credential.attributes.len());
        let msgs = credential
            .attributes
            .iter()
            .map(|a| a.to_signature_message())
            .collect::<Vec<Message>>();
        let res = credential.signature.verify(&vk, &generators, &msgs);
        res.unwrap_u8() == 1
    }

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation.
    ///
    /// Each credential maps to a presentation manifest
    pub fn present_credentials(
        &self,
        credential: &[Credential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: [u8; 32],
        mut rng: impl RngCore + CryptoRng,
    ) -> Result<Vec<CredentialPresentation>, CredentialError> {
        // To prove the id-secret is the same across credentials we use a Schnorr proof
        // which requires that the proof blinding factor be the same. If there's only one credential
        // it makes no difference
        let id_bf = Nonce::random(&mut rng);

        let mut commitments = Vec::new();
        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();

        for (cred, pm) in credential.iter().zip(presentation_manifests.iter()) {
            let mut messages = Vec::new();
            let verkey = PublicKey::from_bytes(&pm.public_key).unwrap();
            let generators = MessageGenerators::from_public_key(verkey, cred.attributes.len());
            // let pr = bbs::prelude::Verifier::new_proof_request(pm.revealed.as_slice(), &verkey)
            //     .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;

            let revealed_indices = pm.revealed.iter().copied().collect::<HashSet<usize>>();
            for i in 0..cred.attributes.len() {
                if pm.credential_schema.attributes[i].label == SECRET_ID {
                    if revealed_indices.contains(&i) {
                        return Err(CredentialError::InvalidPresentationManifest);
                    }
                    messages.push(ProofMessage::Hidden(HiddenMessage::ExternalBlinding(
                        self.id, id_bf,
                    )));
                } else if revealed_indices.contains(&i) {
                    messages.push(ProofMessage::Revealed(
                        cred.attributes[i].to_signature_message(),
                    ));
                } else {
                    messages.push(ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(
                        cred.attributes[i].to_signature_message(),
                    )));
                }
            }

            let mut pok =
                Prover::commit_signature_pok(cred.signature, &generators, &messages, &mut rng)
                    .map_err(|_| CredentialError::MismatchedAttributeClaimType)?;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);
            pok.add_proof_contribution(&mut hasher);
            bytes = hasher.finalize();
            commitments.push(pok);
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        hasher.update(&proof_request_id);
        let challenge = Challenge::hash(&hasher.finalize());
        let presentation_id = challenge.to_bytes();

        let mut proofs = Vec::new();
        for i in 0..commitments.len() {
            let pok = commitments.remove(0);
            let cred = &credential[i];
            let pm = &presentation_manifests[i];

            proofs.push(CredentialPresentation {
                presentation_id,
                revealed_attributes: pm
                    .revealed
                    .iter()
                    .map(|r| cred.attributes[*r].clone())
                    .collect(),
                proof: pok
                    .generate_proof(challenge)
                    .map_err(|_| CredentialError::InvalidPresentationManifest)?,
            });
        }

        Ok(proofs)
    }
}
