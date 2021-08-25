use crate::{
    AuthenticationProof, BbsCredential, Changes, Contact, Credential, CredentialAttribute,
    CredentialFragment1, CredentialFragment2, CredentialOffer, CredentialPresentation,
    CredentialProof, CredentialPublicKey, CredentialRequest, CredentialSchema, EntityCredential,
    Lease, OfferId, PresentationManifest, ProfileChangeEvent, ProfileIdentifier, ProofRequestId,
    TTL,
};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::{Address, Route};
use serde::{Deserialize, Serialize};

pub type EventAttribute = (String, String);
pub type EventAttributes = Vec<EventAttribute>;
pub type ByteVec = Vec<u8>;
pub type Id = ProfileIdentifier;

#[derive(Clone, Serialize, Deserialize)]
pub enum IdentityRequest {
    CreateProfile(Address),
    CreateAuthenticationProof(Id, ByteVec),
    CreateKey(Id, String),
    GetProfilePublicKey(Id),
    GetProfileSecretKey(Id),
    GetPublicKey(Id, String),
    GetSecretKey(Id, String),
    GetChanges(Id),
    GetContacts(Id),
    GetContact(Id, Id),
    RotateKey(Id),
    AddChange(Id, ProfileChangeEvent),
    VerifyAuthenticationProof(Id, ByteVec, Id, AuthenticationProof),
    VerifyChanges(Id),
    VerifyAndAddContact(Id, Contact),
    VerifyContact(Id, Contact),
    VerifyAndUpdateContact(Id, Id, Changes),
    RemoveProfile(Id),
    CreateSecureChannelListener(Id, Address, Address),
    CreateSecureChannel(Id, Route, Address),
    GetSigningKey(Id),
    GetIssuerPublicKey(Id),
    CreateOffer(Id, CredentialSchema),
    CreateProofOfPossession(Id),
    SignCredential(Id, CredentialSchema, Vec<CredentialAttribute>),
    SignCredentialRequest(
        Id,
        CredentialRequest,
        CredentialSchema,
        Vec<(String, CredentialAttribute)>,
        OfferId,
    ),
    AcceptCredentialOffer(Id, CredentialOffer, CredentialPublicKey),
    CombineCredentialFragments(Id, CredentialFragment1, CredentialFragment2),
    IsValidCredential(Id, BbsCredential, CredentialPublicKey),
    PresentCredential(Id, BbsCredential, PresentationManifest, ProofRequestId),
    CreateProofRequestId(Id),
    VerifyProofOfPossession(Id, CredentialPublicKey, CredentialProof),
    VerifyCredentialPresentation(
        Id,
        CredentialPresentation,
        PresentationManifest,
        ProofRequestId,
    ),
    AddCredential(Id, EntityCredential),
    GetCredential(Id, Credential),
    GetLease(Route, Id, String, String, TTL),
    RevokeLease(Route, Id, Lease),
}
