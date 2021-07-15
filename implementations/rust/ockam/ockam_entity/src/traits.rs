use crate::credential::{CredentialOffer, CredentialRequest, SigningPublicKey};
use crate::{
    BbsCredential, Changes, Contact, Credential, CredentialAttribute, CredentialFragment1,
    CredentialFragment2, CredentialPresentation, CredentialProof, CredentialPublicKey,
    CredentialRequestFragment, CredentialSchema, EntityCredential, OfferId, PresentationManifest,
    ProfileChangeEvent, ProfileIdentifier, ProofRequestId, TrustPolicy,
};
use ockam_core::{Address, Result, Route};
use ockam_vault_core::{PublicKey, Secret};
use signature_bls::SecretKey;

pub type AuthenticationProof = Vec<u8>;

/// Identity
pub trait Identity: Send + 'static {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> Result<ProfileIdentifier>;

    /// Create new key.
    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()>;

    /// Rotate existing key.
    fn rotate_profile_key(&mut self) -> Result<()>;

    /// Get [`Secret`] key.
    fn get_profile_secret_key(&self) -> Result<Secret>;

    /// Get [`Secret`] key.
    fn get_secret_key<S: Into<String>>(&self, label: S) -> Result<Secret>;

    /// Get [`PublicKey`].
    fn get_profile_public_key(&self) -> Result<PublicKey>;

    /// Get [`PublicKey`].
    fn get_public_key<S: Into<String>>(&self, label: S) -> Result<PublicKey>;

    /// Create an authentication proof based on the given state
    fn create_auth_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<AuthenticationProof>;

    /// Verify a proof based on the given state, proof and profile.
    fn verify_auth_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        state_slice: S,
        peer_id: &ProfileIdentifier,
        proof_slice: P,
    ) -> Result<bool>;

    /// Add a change event.
    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()>;

    /// Return change history chain
    fn get_changes(&self) -> Result<Changes>;

    /// Verify the whole change event chain
    fn verify_changes(&mut self) -> Result<bool>;

    /// Return all known to this profile [`Contact`]s
    fn get_contacts(&self) -> Result<Vec<Contact>>;

    /// Convert [`Profile`] to [`Contact`]
    fn as_contact(&mut self) -> Result<Contact>;

    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>>;

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool>;
}

pub trait SecureChannels {
    fn create_secure_channel_listener(
        &mut self,
        address: impl Into<Address> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<()>;

    fn create_secure_channel(
        &mut self,
        route: impl Into<Route> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address>;
}

/// Issuer API
pub trait Issuer {
    /// Return the signing key associated with this CredentialIssuer
    fn get_signing_key(&mut self) -> Result<SecretKey>;

    /// Return the public key
    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey>;

    /// Create a credential offer
    fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer>;

    /// Create a proof of possession for this issuers signing key
    fn create_proof_of_possession(&self) -> Result<CredentialProof>;

    /// Sign the claims into the credential
    fn sign_credential<A: AsRef<[CredentialAttribute]>>(
        &self,
        schema: &CredentialSchema,
        attributes: A,
    ) -> Result<BbsCredential>;

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    fn sign_credential_request<A: AsRef<[(String, CredentialAttribute)]>>(
        &self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: A,
        offer_id: OfferId,
    ) -> Result<CredentialFragment2>;
}

/// Holder API
pub trait Holder {
    fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_public_key: SigningPublicKey,
    ) -> Result<CredentialRequestFragment>;

    /// Combine credential fragments to yield a completed credential
    fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential>;

    /// Check a credential to make sure its valid
    fn is_valid_credential(
        &self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool>;

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation. Each credential maps to a presentation manifest
    fn create_credential_presentation(
        &self,
        credential: &BbsCredential,
        presentation_manifests: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<CredentialPresentation>;

    fn add_credential(&mut self, credential: EntityCredential) -> Result<()>;

    fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential>;
}

/// Verifier API
pub trait Verifier {
    /// Create a unique proof request id so the holder must create a fresh proof
    fn create_proof_request_id(&self) -> Result<ProofRequestId>;

    /// Verify a proof of possession
    fn verify_proof_of_possession(
        &self,
        signing_public_key: CredentialPublicKey,
        proof: CredentialProof,
    ) -> Result<bool>;

    /// Check if the credential presentations are valid
    fn verify_credential_presentation(
        &self,
        presentation: &CredentialPresentation,
        presentation_manifest: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<bool>;
}
