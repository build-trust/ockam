use crate::{Changes, Contact, ProfileIdentifier, Proof};
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
    CreateAuthenticationProof(Proof),
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
}
