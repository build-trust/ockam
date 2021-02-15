use crate::error::CredentialError;
use crate::*;
use bbs::prelude::*;
use ockam_core::lib::*;

/// Represents a holder of a credential
#[derive(Debug)]
pub struct Holder {
    id: SignatureMessage,
}

impl Holder {
    /// Create a new Holder with a new unique id
    pub fn new() -> Self {
        Self {
            id: Prover::new_link_secret(),
        }
    }

    /// Accepts a credential offer from an issuer
    pub fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_pk: &DeterministicPublicKey,
    ) -> Result<(CredentialRequest, SignatureBlinding), CredentialError> {
        let nonce = ProofNonce::from(offer.id);
        let mut i = 0;
        for (j, att) in offer.schema.attributes.iter().enumerate() {
            if att.label == "id-secret" {
                i = j;
                break;
            }
        }

        let pk = issuer_pk
            .to_public_key(offer.schema.attributes.len())
            .map_err(|_| CredentialError::InvalidCredentialSchema)?;
        let mut messages = BTreeMap::new();
        messages.insert(i, self.id.clone());
        let (context, blinding) = Prover::new_blind_signature_context(&pk, &messages, &nonce)
            .map_err(|_| CredentialError::InvalidCredentialOffer)?;
        Ok((CredentialRequest { context }, blinding))
    }

    /// Convert a blinded credential to an unblinded one
    pub fn unblind_credential(
        &self,
        blind_credential: &BlindCredential,
        blinding: &SignatureBlinding,
    ) -> Credential {
        // TODO: figure out the best way to add `self.id` to the credential.attributes
        Credential {
            attributes: blind_credential.attributes.to_vec(),
            signature: blind_credential.signature.to_unblinded(blinding),
        }
    }
}
