use super::*;
use bbs::prelude::{
    DeterministicPublicKey, Issuer as BbsIssuer, KeyGenOption, ProofNonce, RandomElem, SecretKey,
};
use core::convert::TryFrom;
use ockam_core::lib::*;
use pairing_plus::{
    bls12_381::{Fr, G1},
    hash_to_curve::HashToCurve,
    hash_to_field::ExpandMsgXmd,
    serdes::SerDes,
    CurveProjective,
};

pub(crate) const CSUITE_POP: &'static [u8] = b"BLS_POP_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

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
    pub fn new() -> Self {
        Self {
            signing_key: SecretKey::random(),
        }
    }

    /// Return the signing key associated with this CredentialIssuer
    pub fn get_signing_key(&self) -> SigningKeyBytes {
        self.signing_key.to_bytes_compressed_form()
    }

    /// Return the public key
    pub fn get_public_key(&self) -> PublicKeyBytes {
        let (dpk, _) = DeterministicPublicKey::new(Some(KeyGenOption::FromSecretKey(
            self.signing_key.clone(),
        )));
        dpk.to_bytes_compressed_form()
    }

    /// Initialize an issuer with an already generated key
    pub fn with_signing_key(signing_key: SigningKeyBytes) -> Self {
        let signing_key = SecretKey::from(signing_key);
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
    pub fn create_offer(&self, schema: &CredentialSchema) -> CredentialOffer {
        let id = BbsIssuer::generate_signing_nonce().to_bytes_compressed_form();
        CredentialOffer {
            id,
            schema: schema.clone(),
        }
    }

    /// Create a proof of possession for this issuers signing key
    pub fn create_proof_of_possession(&self) -> ProofBytes {
        let mut p = <G1 as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(
            &self.get_public_key(),
            CSUITE_POP,
        );

        let mut c = std::io::Cursor::new(self.signing_key.to_bytes_compressed_form());
        let fr = Fr::deserialize(&mut c, true).unwrap();
        p.mul_assign(fr);
        let mut s = [0u8; 48];
        let _ = p.serialize(&mut s.as_mut(), true);
        s
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

        let (dpk, _) = DeterministicPublicKey::new(Some(KeyGenOption::FromSecretKey(
            self.signing_key.clone(),
        )));
        let pk = dpk
            .to_public_key(schema.attributes.len())
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
        let signature = BbsIssuer::sign(messages.as_slice(), &self.signing_key, &pk)
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
        attributes: &BTreeMap<String, CredentialAttribute>,
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2, CredentialError> {
        if attributes.len() >= schema.attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims);
        }
        // Check if any blinded messages are allowed to be unknown
        // If allowed, proceed
        // otherwise abort
        for att in &schema.attributes {
            // missing schema attribute means it's hidden by the holder
            // or unknown to the issuer
            if let None = attributes.get(&att.label) {
                if !att.unknown {
                    return Err(CredentialError::InvalidCredentialAttribute);
                }
            }
        }

        let atts = schema
            .attributes
            .iter()
            .enumerate()
            .map(|(i, a)| (a.label.clone(), (i, a.clone())))
            .collect::<BTreeMap<String, (usize, CredentialAttributeSchema)>>();
        let mut messages = BTreeMap::new();

        let mut remaining_atts = BTreeMap::new();
        for (label, data) in attributes {
            let (i, a) = atts
                .get(label)
                .ok_or(CredentialError::InvalidCredentialAttribute)?;
            if *data != a.attribute_type {
                return Err(CredentialError::MismatchedAttributeClaimType);
            }
            remaining_atts.insert(*i, data.clone());
            messages.insert(*i, data.to_signature_message());
        }
        let (dpk, _) = DeterministicPublicKey::new(Some(KeyGenOption::FromSecretKey(
            self.signing_key.clone(),
        )));
        let pk = dpk
            .to_public_key(schema.attributes.len())
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;

        let signature = BbsIssuer::blind_sign(
            &ctx.context,
            &messages,
            &self.signing_key,
            &pk,
            &ProofNonce::from(offer_id),
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
    use super::*;

    #[test]
    fn create_proof_of_possession_test() {
        let issuer = CredentialIssuer::new();

        let proof = issuer.create_proof_of_possession();

        let mut t = 0u8;
        for b in &proof {
            t |= *b;
        }
        assert!(t > 0);
    }
}
