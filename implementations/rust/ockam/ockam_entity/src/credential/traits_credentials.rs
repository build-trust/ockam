use crate::credential::{CredentialOffer, CredentialRequest, SigningPublicKey};
use crate::{
    BbsCredential, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialPresentation, CredentialProof, CredentialPublicKey, CredentialRequestFragment,
    CredentialSchema, EntityCredential, OfferId, PresentationManifest, ProofRequestId,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use signature_bls::SecretKey;

/// Issuer API
#[async_trait]
pub trait Issuer {
    /// Return the signing key associated with this CredentialIssuer
    async fn get_signing_key(&mut self) -> Result<SecretKey>;

    /// Return the public key
    async fn get_signing_public_key(&mut self) -> Result<SigningPublicKey>;

    /// Create a credential offer
    async fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer>;

    /// Create a proof of possession for this issuers signing key
    async fn create_proof_of_possession(&self) -> Result<CredentialProof>;

    /// Sign the claims into the credential
    async fn sign_credential(
        &self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<BbsCredential>;

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    async fn sign_credential_request(
        &self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferId,
    ) -> Result<CredentialFragment2>;
}

/// Holder API
#[async_trait]
pub trait Holder {
    async fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_public_key: SigningPublicKey,
    ) -> Result<CredentialRequestFragment>;

    /// Combine credential fragments to yield a completed credential
    async fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential>;

    /// Check a credential to make sure its valid
    async fn is_valid_credential(
        &self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool>;

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation. Each credential maps to a presentation manifest
    async fn create_credential_presentation(
        &self,
        credential: &BbsCredential,
        presentation_manifests: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<CredentialPresentation>;

    /// Add credential that this entity possess
    async fn add_credential(&mut self, credential: EntityCredential) -> Result<()>;

    /// Get credential that this entity possess
    async fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential>;
}

/// Verifier API
#[async_trait]
pub trait Verifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    async fn create_proof_request_id(&self) -> Result<ProofRequestId>;

    /// Verify a proof of possession
    async fn verify_proof_of_possession(
        &self,
        signing_public_key: CredentialPublicKey,
        proof: CredentialProof,
    ) -> Result<bool>;

    /// Check if the credential presentations are valid
    async fn verify_credential_presentation(
        &self,
        presentation: &CredentialPresentation,
        presentation_manifest: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<bool>;
}
