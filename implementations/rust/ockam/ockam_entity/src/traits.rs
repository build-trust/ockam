use crate::{
    Contact, ContactsDb, KeyAttributes, ProfileChangeEvent, ProfileEventAttributes,
    ProfileIdentifier,
};
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret};

pub trait ProfileIdentity {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> Result<ProfileIdentifier>;
}

pub trait ProfileChanges {
    /// Return change history chain
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>>;
    /// Add a change event.
    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()>;
    /// Verify the whole change event chain
    fn verify(&mut self) -> Result<bool>;
}

pub trait ProfileContacts {
    /// Return all known to this profile [`Contact`]s
    fn contacts(&self) -> Result<ContactsDb>;
    /// Convert [`Profile`] to [`Contact`]
    fn to_contact(&self) -> Result<Contact>;
    /// Serialize [`Profile`] to [`Contact`] in binary form for storing/transferring over the network
    fn serialize_to_contact(&self) -> Result<Vec<u8>>;
    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>>;
    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact(&mut self, contact: &Contact) -> Result<bool>;
    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool>;
    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool>;
}

pub trait ProfileAuth {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>>;
    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool>;
}

pub trait ProfileSecrets {
    /// Create new key. Key is uniquely identified by label in [`KeyAttributes`]
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()>;

    /// Rotate existing key. Key is uniquely identified by label in [`KeyAttributes`]
    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()>;

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret>;

    /// Get [`PublicKey`]. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey>;

    /// Get the root [`Secret`]
    fn get_root_secret(&mut self) -> Result<Secret>;
}

/// Supertrait of a Profile
pub trait ProfileTrait:
    ProfileIdentity + ProfileChanges + ProfileSecrets + ProfileContacts + ProfileAuth + Send + 'static
{
}
