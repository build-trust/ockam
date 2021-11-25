use crate::{AuthenticationProof, Changes, Contact, Lease, ProfileIdentifier};
use cfg_if::cfg_if;
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, Message};
use ockam_vault::{PublicKey, Secret};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum MaybeContact {
    None,
    Contact(Contact),
}

#[derive(Serialize, Deserialize, Message)]
pub enum IdentityResponse {
    AddKey,
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
    Lease(Lease),
    #[cfg(feature = "credentials")]
    CredentialResponse(IdentityCredentialResponse),
}

cfg_if! {
    if #[cfg(feature = "credentials")] {
        use crate::{
            BlsSecretKey,
            BbsCredential, CredentialFragment2, CredentialOffer, CredentialPresentation, CredentialProof,
            CredentialPublicKey, CredentialRequestFragment, EntityCredential, ProofRequestId,
        };

        #[derive(Serialize, Deserialize)]
        pub enum IdentityCredentialResponse {
            GetSigningKey(BlsSecretKey),
            GetIssuerPublicKey(CredentialPublicKey),
            CreateOffer(CredentialOffer),
            CreateProofOfPossession(CredentialProof),
            SignCredential(BbsCredential),
            SignCredentialRequest(CredentialFragment2),
            AcceptCredentialOffer(CredentialRequestFragment),
            CombineCredentialFragments(BbsCredential),
            IsValidCredential(bool),
            PresentCredential(CredentialPresentation),
            CreateProofRequestId(ProofRequestId),
            VerifyProofOfPossession(bool),
            VerifyCredentialPresentation(bool),
            AddCredential,
            GetCredential(EntityCredential),
        }

    }
}
