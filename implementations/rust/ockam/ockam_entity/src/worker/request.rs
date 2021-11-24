use crate::{
    AuthenticationProof, Changes, Contact, Lease, ProfileChangeEvent, ProfileIdentifier, TTL,
};
use cfg_if::cfg_if;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::{Address, Message, Route};
use ockam_vault_core::Secret;
use serde::{Deserialize, Serialize};

pub type EventAttribute = (String, String);
pub type EventAttributes = Vec<EventAttribute>;
pub type ByteVec = Vec<u8>;
pub type Id = ProfileIdentifier;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Serialize, Deserialize, Message)]
pub enum IdentityRequest {
    CreateProfile(Address),
    CreateAuthenticationProof(Id, ByteVec),
    CreateKey(Id, String),
    AddKey(Id, String, Secret),
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
    GetLease(Route, Id, String, String, TTL),
    RevokeLease(Route, Id, Lease),
    #[cfg(feature = "credentials")]
    CredentialRequest(IdentityCredentialRequest),
}

cfg_if! {
    if #[cfg(feature = "credentials")] {
        use crate::{
            BbsCredential, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
            CredentialOffer, CredentialPresentation, CredentialProof, CredentialPublicKey,
            CredentialRequest, CredentialSchema, EntityCredential, OfferId, PresentationManifest,
            ProofRequestId,
        };

        #[derive(Clone, Serialize, Deserialize)]
        pub enum IdentityCredentialRequest {
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
        }
    }
}
