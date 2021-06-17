use crate::{Changes, Contact, ProfileChangeEvent, ProfileIdentifier, Proof};
use ockam_core::{Address, Route};
use serde::{Deserialize, Serialize};

pub type EventAttribute = (String, String);
pub type EventAttributes = Vec<EventAttribute>;
pub type ByteVec = Vec<u8>;
pub type Id = ProfileIdentifier;

#[derive(Clone, Serialize, Deserialize)]
pub enum IdentityRequest {
    CreateProfile,
    CreateAuthenticationProof(Id, ByteVec),
    CreateKey(Id, String),

    GetPublicKey(Id),
    GetSecretKey(Id),
    GetChanges(Id),
    GetContacts(Id),
    GetContact(Id, Id),

    RotateKey(Id),
    AddChange(Id, ProfileChangeEvent),

    VerifyAuthenticationProof(Id, ByteVec, Id, Proof),
    VerifyChanges(Id),
    VerifyAndAddContact(Id, Contact),
    VerifyContact(Id, Contact),
    VerifyAndUpdateContact(Id, Id, Changes),
    RemoveProfile(Id),
    CreateSecureChannelListener(Id, Address),
    CreateSecureChannel(Id, Route),
}
