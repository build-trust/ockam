use super::*;
use bbs::prelude::*;
use digest::{generic_array::GenericArray, Digest, FixedOutput};
use ockam_core::lib::*;

/// The label to indicate the secretid attribute in a schema/credential
pub const SECRET_ID: &'static str = "secret_id";

/// Represents a holder of a credential
#[derive(Debug)]
pub struct CredentialHolder {
    pub(crate) id: SignatureMessage,
}

impl CredentialHolder {
    /// Create a new CredentialHolder with a new unique id
    pub fn new() -> Self {
        Self {
            id: Prover::new_link_secret(),
        }
    }

    /// Accepts a credential offer from an issuer
    pub fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_pk: [u8; 96],
    ) -> Result<(CredentialRequest, CredentialFragment1), CredentialError> {
        let nonce = ProofNonce::from(offer.id);
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

        let dpk = DeterministicPublicKey::from(issuer_pk);
        let pk = dpk
            .to_public_key(offer.schema.attributes.len())
            .map_err(|_| CredentialError::InvalidCredentialSchema)?;
        let mut messages = BTreeMap::new();
        messages.insert(i, self.id.clone());
        let (context, blinding) = Prover::new_blind_signature_context(&pk, &messages, &nonce)
            .map_err(|_| CredentialError::InvalidCredentialOffer)?;
        Ok((
            CredentialRequest { context },
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
                attributes.insert(
                    i,
                    CredentialAttribute::Blob(self.id.to_bytes_compressed_form()),
                );
                break;
            }
        }
        Credential {
            attributes,
            signature: credential_fragment2
                .signature
                .to_unblinded(&credential_fragment1.blinding),
        }
    }

    /// Check a credential to make sure its valid
    pub fn is_valid_credential(&self, credential: &Credential, verkey: [u8; 96]) -> bool {
        // credential cannot have zero attributes so unwrap is okay
        let vk = DeterministicPublicKey::from(verkey)
            .to_public_key(credential.attributes.len())
            .unwrap();
        let msgs = credential
            .attributes
            .iter()
            .map(|a| a.to_signature_message())
            .collect::<Vec<SignatureMessage>>();
        let res = credential.signature.verify(msgs.as_slice(), &vk);
        res.unwrap_or_else(|_| false)
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
    ) -> Result<Vec<CredentialPresentation>, CredentialError> {
        // To prove the id-secret is the same across credentials we use a Schnorr proof
        // which requires that the proof blinding proof be the same. If there's only one credential
        // it makes no difference
        let id_bf = ProofNonce::random();

        let mut commitments = Vec::new();
        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();

        for (cred, pm) in credential.iter().zip(presentation_manifests.iter()) {
            let mut messages = Vec::new();
            let dpk = DeterministicPublicKey::from(pm.public_key);
            let verkey = dpk
                .to_public_key(pm.credential_schema.attributes.len())
                .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
            let pr = bbs::prelude::Verifier::new_proof_request(pm.revealed.as_slice(), &verkey)
                .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
            let revealed_indices = pm.revealed.iter().map(|i| *i).collect::<BTreeSet<usize>>();
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

            let pok = Prover::commit_signature_pok(&pr, messages.as_slice(), &cred.signature)
                .map_err(|_| CredentialError::MismatchedAttributeClaimType)?;
            let mut hasher = sha2::Sha256::new();
            hasher.input(&bytes);
            hasher.input(pok.to_bytes());
            bytes = hasher.result();
            commitments.push(pok);
        }

        let mut hasher = sha2::Sha256::new();
        hasher.input(&bytes);
        hasher.input(&proof_request_id);
        let challenge = ProofChallenge::hash(&hasher.result());
        let presentation_id = challenge.to_bytes_compressed_form();

        let mut proofs = Vec::new();
        for i in 0..commitments.len() {
            let pok = commitments[i].clone();
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
                    .gen_proof(&challenge)
                    .map_err(|_| CredentialError::InvalidPresentationManifest)?,
            });
        }

        Ok(proofs)
    }
}
