use crate::{
    Contact, ContactsDb, Credential, CredentialFragment1, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialPublicKey, CredentialRequest, ProfileChangeEvent,
    ProfileIdentifier, Proof, ProofRequestId, SigningKeyBytes,
};
use ockam_vault_core::{PublicKey, Secret};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProfileResponseMessage {
    Identifier(ProfileIdentifier),
    ChangeEvents(Vec<ProfileChangeEvent>),
    UpdateNoVerification,
    Verify(bool),
    Contacts(ContactsDb),
    ToContact(Contact),
    SerializeToContact(Vec<u8>),
    GetContact(Option<Contact>),
    VerifyContact(bool),
    VerifyAndAddContact(bool),
    VerifyAndUpdateContact(bool),
    GenerateAuthenticationProof(Vec<u8>),
    VerifyAuthenticationProof(bool),
    CreateKey,
    RotateKey,
    GetSecretKey(Secret),
    GetPublicKey(PublicKey),
    GetRootSecret(Secret),
    // Issuer traits
    GetSigningKey(SigningKeyBytes),
    GetIssuerPublicKey(CredentialPublicKey),
    CreateOffer(CredentialOffer),
    CreateProofOfPossession(Proof),
    SignCredential(Credential),
    SignCredentialRequest(CredentialFragment2),
    // Holder traits
    AcceptCredentialOffer((CredentialRequest, CredentialFragment1)),
    CombineCredentialFragments(Credential),
    IsValidCredential(bool),
    PresentCredentials(Vec<CredentialPresentation>),
    // Verifier traits
    CreateProofRequestId(ProofRequestId),
    VerifyProofOfPossession(bool),
    VerifyCredentialPresentation(bool),
}
