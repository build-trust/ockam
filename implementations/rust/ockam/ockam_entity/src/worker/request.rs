use crate::{
    Contact, KeyAttributes, ProfileChangeEvent, ProfileEventAttributes, ProfileIdentifier,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProfileRequestMessage {
    Identifier,
    ChangeEvents,
    UpdateNoVerification {
        change_event: ProfileChangeEvent,
    },
    Verify,
    Contacts,
    ToContact,
    SerializeToContact,
    GetContact {
        id: ProfileIdentifier,
    },
    VerifyContact {
        contact: Contact,
    },
    VerifyAndAddContact {
        contact: Contact,
    },
    VerifyAndUpdateContact {
        profile_id: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    },
    GenerateAuthenticationProof {
        channel_state: Vec<u8>,
    },
    VerifyAuthenticationProof {
        channel_state: Vec<u8>,
        responder_contact_id: ProfileIdentifier,
        proof: Vec<u8>,
    },
    CreateKey {
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    },
    RotateKey {
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    },
    GetSecretKey {
        key_attributes: KeyAttributes,
    },
    GetPublicKey {
        key_attributes: KeyAttributes,
    },
    GetRootSecret,
}
