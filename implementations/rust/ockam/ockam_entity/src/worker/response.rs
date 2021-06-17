use crate::{
    AuthenticationProof, Changes, Contact, Credential, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialProof, CredentialPublicKey, CredentialRequestFragment,
    ProfileIdentifier, ProofRequestId, SigningKey,
};
use ockam_core::Address;
use ockam_vault::{PublicKey, Secret};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum MaybeContact {
    None,
    Contact(Contact),
}

#[derive(Serialize, Deserialize)]
pub enum IdentityResponse {
    CreateProfile(ProfileIdentifier),
    CreateAuthenticationProof(AuthenticationProof),
    GetPublicKey(PublicKey),
    GetSecretKey(Secret),
    GetChanges(Changes),
    Contacts(Vec<Contact>),
    GetContact(MaybeContact),
    VerifyAuthenticationProof(bool),
    VerifyContact(bool),
    VerifyAndUpdateContact(bool),
    VerifyChanges(bool),
    VerifyAndAddContact(bool),
    CreateSecureChannel(Address),
    GetSigningKey(SigningKey),
    GetIssuerPublicKey(CredentialPublicKey),
    CreateOffer(CredentialOffer),
    CreateProofOfPossession(CredentialProof),
    SignCredential(Credential),
    SignCredentialRequest(CredentialFragment2),
    AcceptCredentialOffer(CredentialRequestFragment),
    CombineCredentialFragments(Credential),
    IsValidCredential(bool),
    PresentCredential(CredentialPresentation),
    CreateProofRequestId(ProofRequestId),
    VerifyProofOfPossession(bool),
    VerifyCredentialPresentation(bool),
}
