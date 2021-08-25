use crate::{
    BbsCredential, BlsSecretKey, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialRequest, CredentialSchema, EntityError,
    EntityIdentifier, OfferId, PresentationManifest, ProfileIdentifier, ProofBytes, ProofRequestId,
    SigningPublicKey, TrustPolicy,
};
use core::convert::TryFrom;
use core::fmt::{Display, Formatter};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::{hex, Address, Error, Result, Route};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

/// Credential Issuer
pub trait CredentialIssuer {
    /// Return the signing key associated with this CredentialIssuer
    fn get_signing_key(&mut self) -> Result<BlsSecretKey>;

    /// Return the public key
    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey>;

    /// Create a credential offer
    fn create_offer(&mut self, schema: &CredentialSchema) -> Result<CredentialOffer>;

    /// Create a proof of possession for this issuers signing key
    fn create_proof_of_possession(&mut self) -> Result<ProofBytes>;

    /// Sign the claims into the credential
    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<BbsCredential>;

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferId,
    ) -> Result<CredentialFragment2>;
}

/// Credential Holder
pub trait CredentialHolder {
    /// Accepts a credential offer from an issuer
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        signing_public_key: SigningPublicKey,
    ) -> Result<(CredentialRequest, CredentialFragment1)>;

    /// Combine credential fragments to yield a completed credential
    fn combine_credential_fragments(
        &mut self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential>;

    /// Check a credential to make sure its valid
    fn is_valid_credential(
        &mut self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool>;

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation. Each credential maps to a presentation manifest
    fn present_credentials(
        &mut self,
        credential: &[BbsCredential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<Vec<CredentialPresentation>>;
}

/// Credential Verifier
pub trait CredentialVerifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    fn create_proof_request_id(&mut self) -> Result<ProofRequestId>;

    /// Verify a proof of possession
    fn verify_proof_of_possession(
        &mut self,
        issuer_vk: SigningPublicKey,
        proof: ProofBytes,
    ) -> Result<bool>;

    /// Check if the credential presentations are valid
    fn verify_credential_presentations(
        &mut self,
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<bool>;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub struct CredentialIdentifier(String);

impl Distribution<CredentialIdentifier> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CredentialIdentifier {
        let id: [u8; 16] = rng.gen();
        CredentialIdentifier(hex::encode(id))
    }
}

impl CredentialIdentifier {
    pub const PREFIX: &'static str = "C";
}

impl Display for CredentialIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let str: String = self.clone().into();
        write!(f, "{}", &str)
    }
}

impl Into<String> for CredentialIdentifier {
    fn into(self) -> String {
        format!("{}{}", Self::PREFIX, &self.0)
    }
}

impl TryFrom<&str> for CredentialIdentifier {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        if let Some(str) = value.strip_prefix(Self::PREFIX) {
            Ok(Self(str.into()))
        } else {
            Err(EntityError::InvalidProfileId.into())
        }
    }
}

impl TryFrom<String> for CredentialIdentifier {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::try_from(value.as_str())
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Credential {
    id: CredentialIdentifier,
    issuer_id: EntityIdentifier,
    type_id: String,
}

impl Credential {
    pub fn new(id: CredentialIdentifier, issuer_id: EntityIdentifier, type_id: String) -> Self {
        Credential {
            id,
            issuer_id,
            type_id,
        }
    }
}

impl Credential {
    pub fn id(&self) -> &CredentialIdentifier {
        &self.id
    }
    pub fn issuer_id(&self) -> &EntityIdentifier {
        &self.issuer_id
    }
    pub fn type_id(&self) -> &str {
        &self.type_id
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EntityCredential {
    credential: Credential,
    bbs_credential: BbsCredential,
    #[serde(with = "BigArray")]
    issuer_pubkey: SigningPublicKey,
    schema: CredentialSchema,
}

impl EntityCredential {
    pub fn credential(&self) -> &Credential {
        &self.credential
    }
    pub fn bbs_credential(&self) -> &BbsCredential {
        &self.bbs_credential
    }
    pub fn issuer_pubkey(&self) -> SigningPublicKey {
        self.issuer_pubkey
    }
    pub fn schema(&self) -> &CredentialSchema {
        &self.schema
    }
}

impl EntityCredential {
    pub fn new(
        credential: Credential,
        bbs_credential: BbsCredential,
        issuer_pubkey: SigningPublicKey,
        schema: CredentialSchema,
    ) -> Self {
        EntityCredential {
            credential,
            bbs_credential,
            issuer_pubkey,
            schema,
        }
    }
}

pub trait CredentialProtocol {
    fn create_credential_issuance_listener(
        &mut self,
        address: impl Into<Address> + Send,
        schema: CredentialSchema,
        trust_policy: impl TrustPolicy,
    ) -> Result<()>;

    fn acquire_credential(
        &mut self,
        issuer_route: Route,
        issuer_id: &ProfileIdentifier,
        schema: CredentialSchema,
        values: Vec<CredentialAttribute>,
    ) -> Result<Credential>;

    fn present_credential(
        &mut self,
        worker_route: Route,
        credential: Credential,
        reveal_attributes: Vec<String>,
    ) -> Result<()>;

    fn verify_credential(
        &mut self,
        address: impl Into<Address> + Send,
        issuer_id: &ProfileIdentifier,
        schema: CredentialSchema,
        attributes_values: Vec<CredentialAttribute>,
    ) -> Result<bool>;
}
