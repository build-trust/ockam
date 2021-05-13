use crate::{Contact, ContactsDb, ProfileChangeEvent, ProfileIdentifier};
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
}
