use crate::{
    AuthenticationProof, BlsSecretKey, Changes, Contact, Credential, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialProof, CredentialPublicKey,
    CredentialRequestFragment, ProfileIdentifier, ProofRequestId,
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
    GetProfilePublicKey(PublicKey),
    GetProfileSecretKey(Secret),
    GetSecretKey(Secret),
    GetChanges(Changes),
    Contacts(Vec<Contact>),
    GetContact(MaybeContact),
    VerifyAuthenticationProof(bool),
    VerifyContact(bool),
    VerifyAndUpdateContact(bool),
    VerifyChanges(bool),
    VerifyAndAddContact(bool),
    CreateSecureChannelListener,
    CreateSecureChannel(Address),
    GetSigningKey(BlsSecretKey),
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
