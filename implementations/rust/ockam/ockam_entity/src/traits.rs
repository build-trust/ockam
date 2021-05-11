use crate::{
    Contact, ContactsDb, KeyAttributes, ProfileChangeEvent, ProfileEventAttributes,
    ProfileIdentifier,
};
use ockam_vault_core::{PublicKey, Secret};

pub trait ProfileIdentity {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> &ProfileIdentifier;
}

pub trait ProfileChanges {
    /// Return change history chain
    fn change_events(&self) -> &[ProfileChangeEvent];
    /// Add a change event.
    fn update_no_verification(
        &mut self,
        change_event: ProfileChangeEvent,
    ) -> ockam_core::Result<()>;

    /// Verify the whole change event chain
    fn verify(&mut self) -> ockam_core::Result<()>;
}

pub trait ProfileContacts {
    /// Return all known to this profile [`Contact`]s
    fn contacts(&self) -> &ContactsDb;
    /// Convert [`Profile`] to [`Contact`]
    fn to_contact(&self) -> Contact;
    /// Serialize [`Profile`] to [`Contact`] in binary form for storing/transferring over the network
    fn serialize_to_contact(&self) -> ockam_core::Result<Vec<u8>>;
    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&self, id: &ProfileIdentifier) -> Option<&Contact>;
    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact(&mut self, contact: &Contact) -> ockam_core::Result<()>;
    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    fn verify_and_add_contact(&mut self, contact: Contact) -> ockam_core::Result<()>;
    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> ockam_core::Result<()>;
}

pub trait ProfileAuth {
    fn generate_authentication_proof(
        &mut self,
        channel_state: &[u8],
    ) -> ockam_core::Result<Vec<u8>>;
    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> ockam_core::Result<bool>;
}

pub trait ProfileSecrets {
    /// Create new key. Key is uniquely identified by label in [`KeyAttributes`]
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()>;

    /// Rotate existing key. Key is uniquely identified by label in [`KeyAttributes`]
    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()>;

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> ockam_core::Result<Secret>;

    /// Get [`PublicKey`]. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_public_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<PublicKey>;

    /// Get the root [`Secret`]
    fn get_root_secret(&mut self) -> ockam_core::Result<Secret>;
}

/// Supertrait of a Profile
pub trait ProfileTrait:
    ProfileIdentity + ProfileChanges + ProfileSecrets + ProfileContacts + ProfileAuth
{
}
