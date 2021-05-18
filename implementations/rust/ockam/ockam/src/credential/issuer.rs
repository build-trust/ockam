use super::*;

use core::convert::TryFrom;
use ockam_core::lib::{HashMap, Result, String, Vec};
use rand::{CryptoRng, RngCore};
use signature_bbs_plus::{Issuer as BbsIssuer, MessageGenerators, PublicKey, SecretKey};
use signature_bls::ProofOfPossession;
use signature_core::lib::*;

/// Represents an issuer of a credential
#[derive(Debug)]
pub struct CredentialIssuer {
    signing_key: SecretKey,
}

/// Alias for an array of 32 bytes.
pub type SigningKeyBytes = [u8; 32];

/// Alias for an array of 96 bytes.
pub type PublicKeyBytes = [u8; 96];

/// Alias for an array of 48 bytes.
pub type ProofBytes = [u8; 48];

/// Alias for an array of 32 bytes.
pub type OfferIdBytes = [u8; 32];

impl CredentialIssuer {
    /// Create issuer with a new issuing key
    pub fn new(rng: impl RngCore + CryptoRng) -> Self {
        Self {
            signing_key: SecretKey::random(rng).expect("bad random number generator"),
        }
    }

    /// Return the signing key associated with this CredentialIssuer
    pub fn get_signing_key(&self) -> SigningKeyBytes {
        self.signing_key.to_bytes()
    }

    /// Return the public key
    pub fn get_public_key(&self) -> PublicKeyBytes {
        let pk = PublicKey::from(&self.signing_key);
        pk.to_bytes()
    }

    /// Initialize an issuer with an already generated key
    pub fn with_signing_key(signing_key: SigningKeyBytes) -> Self {
        let signing_key = SecretKey::from_bytes(&signing_key).unwrap();
        Self { signing_key }
    }

    /// Initialize an issuer with a hex encoded, already generated key
    pub fn with_signing_key_hex(signing_key_hex: String) -> Option<Self> {
        if let Ok(key) = ockam_core::hex::decode(signing_key_hex) {
            if let Ok(key) = <SigningKeyBytes>::try_from(key.as_slice()) {
                Some(CredentialIssuer::with_signing_key(key))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Create a credential offer
    pub fn create_offer(
        &self,
        schema: &CredentialSchema,
        rng: impl RngCore + CryptoRng,
    ) -> CredentialOffer {
        let id = Nonce::random(rng).to_bytes();
        CredentialOffer {
            id,
            schema: schema.clone(),
        }
    }

    /// Create a proof of possession for this issuers signing key
    pub fn create_proof_of_possession(&self) -> ProofBytes {
        ProofOfPossession::new(&self.signing_key)
            .expect("bad signing key")
            .to_bytes()
    }

    /// Sign the claims into the credential
    pub fn sign_credential(
        &self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<Credential, CredentialError> {
        if schema.attributes.len() != attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims);
        }
        let mut messages = Vec::new();
        for (att, v) in schema.attributes.iter().zip(attributes) {
            match (att.attribute_type, v) {
                (CredentialAttributeType::Blob, CredentialAttribute::Blob(_)) => {
                    messages.push(v.to_signature_message())
                }
                (CredentialAttributeType::Utf8String, CredentialAttribute::String(_)) => {
                    messages.push(v.to_signature_message())
                }
                (CredentialAttributeType::Number, CredentialAttribute::Numeric(_)) => {
                    messages.push(v.to_signature_message())
                }
                (_, CredentialAttribute::NotSpecified) => messages.push(v.to_signature_message()),
                (_, CredentialAttribute::Empty) => messages.push(v.to_signature_message()),
                (_, _) => return Err(CredentialError::MismatchedAttributeClaimType),
            }
        }

        let generators =
            MessageGenerators::from_secret_key(&self.signing_key, schema.attributes.len());

        let signature = BbsIssuer::sign(&self.signing_key, &generators, &messages)
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
        Ok(Credential {
            attributes: attributes.to_vec(),
            signature,
        })
    }

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    pub fn sign_credential_request(
        &self,
        ctx: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2, CredentialError> {
        if attributes.len() >= schema.attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims);
        }

        let mut atts = HashMap::new();

        for (name, att) in attributes {
            atts.insert(name, att);
        }

        let mut messages = Vec::<(usize, Message)>::new();
        let mut remaining_atts = Vec::<(usize, CredentialAttribute)>::new();

        // Check if any blinded messages are allowed to be unknown
        // If allowed, proceed
        // otherwise abort
        for i in 0..schema.attributes.len() {
            let att = &schema.attributes[i];
            // missing schema attribute means it's hidden by the holder
            // or unknown to the issuer
            match atts.get(&att.label) {
                None => {
                    if !att.unknown {
                        return Err(CredentialError::InvalidCredentialAttribute);
                    }
                }
                Some(data) => {
                    if **data != att.attribute_type {
                        return Err(CredentialError::MismatchedAttributeClaimType);
                    }
                    remaining_atts.push((i, (*data).clone()));
                    messages.push((i, (*data).to_signature_message()));
                }
            }
        }

        let generators =
            MessageGenerators::from_secret_key(&self.signing_key, schema.attributes.len());

        let signature = BbsIssuer::blind_sign(
            &ctx.context,
            &self.signing_key,
            &generators,
            &messages,
            Nonce::from_bytes(&offer_id).unwrap(),
        )
        .map_err(|_| CredentialError::InvalidCredentialAttribute)?;

        Ok(CredentialFragment2 {
            attributes: remaining_atts.iter().map(|(_, v)| v.clone()).collect(),
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::util::MockRng;
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn create_proof_of_possession_test() {
        let seed = [3u8; 16];
        let mut rng = MockRng::from_seed(seed);
        let issuer = CredentialIssuer::new(&mut rng);

        let proof = issuer.create_proof_of_possession();

        let mut t = 0u8;
        for b in &proof {
            t |= *b;
        }
        assert!(t > 0);
    }
}
