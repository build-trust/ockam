use crate::{
    Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialRequest, CredentialSchema, OfferIdBytes,
    PresentationManifest, ProofBytes, ProofRequestId, PublicKeyBytes, SigningKeyBytes,
};
use ockam_core::Result;
use rand::{CryptoRng, RngCore};

/// Credential Issuer
pub trait CredentialIssuer {
    /// Return the signing key associated with this CredentialIssuer
    fn get_signing_key(&mut self) -> Result<SigningKeyBytes>;

    /// Return the public key
    fn get_issuer_public_key(&mut self) -> Result<PublicKeyBytes>;

    /// Create a credential offer
    fn create_offer(
        &mut self,
        schema: &CredentialSchema,
        rng: impl RngCore + CryptoRng,
    ) -> Result<CredentialOffer>;

    /// Create a proof of possession for this issuers signing key
    fn create_proof_of_possession(&mut self) -> Result<ProofBytes>;

    /// Sign the claims into the credential
    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<Credential>;

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2>;
}

/// Credential Holder
pub trait CredentialHolder {
    /// Accepts a credential offer from an issuer
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        issuer_pk: PublicKeyBytes,
        rng: impl RngCore + CryptoRng,
    ) -> Result<(CredentialRequest, CredentialFragment1)>;

    /// Combine credential fragments to yield a completed credential
    fn combine_credential_fragments(
        &mut self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<Credential>;

    /// Check a credential to make sure its valid
    fn is_valid_credential(
        &mut self,
        credential: &Credential,
        verifier_key: PublicKeyBytes,
    ) -> Result<bool>;

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation. Each credential maps to a presentation manifest
    fn present_credentials(
        &mut self,
        credential: &[Credential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
        rng: impl RngCore + CryptoRng,
    ) -> Result<Vec<CredentialPresentation>>;
}

/// Credential Verifier
pub trait CredentialVerifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    fn create_proof_request_id(&mut self, rng: impl RngCore) -> Result<ProofRequestId>;

    /// Verify a proof of possession
    fn verify_proof_of_possession(
        &mut self,
        issuer_vk: PublicKeyBytes,
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
