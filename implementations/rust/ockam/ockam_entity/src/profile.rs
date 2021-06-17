/// Profile is an abstraction responsible for keeping, verifying and modifying
/// user's data (mainly - public keys). It is used to create new keys, rotate and revoke them.
/// Public keys together with metadata will be organised into events chain, corresponding
/// secret keys will be saved into the given Vault implementation. Events chain and corresponding
/// secret keys are what fully determines Profile.
///
///
/// # Examples
///
/// Create a [`Profile`]. Add and rotate keys.
/// TODO
///
/// Authentication using [`Profile`]. In following example Bob authenticates Alice.
/// TODO
///
/// Update [`Profile`] and send changes to other parties. In following example Alice rotates
/// her key and sends corresponding [`Profile`] changes to Bob.
/// TODO
///
use crate::{
    Changes, Contact, Entity, Handle, Identity, ProfileChangeEvent, ProfileIdentifier, Proof,
    SecureChannels,
};
use ockam_core::{Address, Result, Route};
use ockam_vault::{PublicKey, Secret};

#[derive(Clone)]
pub struct Profile {
    id: ProfileIdentifier,
    handle: Handle,
}

impl From<Profile> for Entity {
    fn from(p: Profile) -> Entity {
        Entity::new(p.handle.clone(), p.id.clone())
    }
}

impl Profile {
    pub fn new<I: Into<ProfileIdentifier>>(id: I, handle: Handle) -> Self {
        let id = id.into();
        Profile { id, handle }
    }

    pub fn entity(&self) -> Entity {
        Entity::from(self.clone())
    }
}

impl Profile {
    /// Sha256 of that value is used as previous event id for first event in a [`Profile`]
    pub const NO_EVENT: &'static [u8] = "OCKAM_NO_EVENT".as_bytes();
    /// Label for [`Profile`] update key
    pub const PROFILE_UPDATE: &'static str = "OCKAM_PUK";
    /// Label for key used to issue credentials
    pub const CREDENTIALS_ISSUE: &'static str = "OCKAM_CIK";
    /// Current version of change structure
    pub const CURRENT_CHANGE_VERSION: u8 = 1;
}

impl Identity for Profile {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        self.entity().identifier()
    }

    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()> {
        self.entity().create_key(label)
    }

    fn rotate_key(&mut self) -> Result<()> {
        self.entity().rotate_key()
    }

    fn get_secret_key(&self) -> Result<Secret> {
        self.entity().get_secret_key()
    }

    fn get_public_key(&self) -> Result<PublicKey> {
        self.entity().get_public_key()
    }

    fn create_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<Proof> {
        self.entity().create_proof(state_slice)
    }

    fn verify_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        state_slice: S,
        peer_id: &ProfileIdentifier,
        proof_slice: P,
    ) -> Result<bool> {
        self.entity()
            .verify_proof(state_slice, peer_id, proof_slice)
    }

    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        self.entity().add_change(change_event)
    }

    fn get_changes(&self) -> Result<Changes> {
        self.entity().get_changes()
    }

    fn verify_changes(&mut self) -> Result<bool> {
        self.entity().verify_changes()
    }

    fn get_contacts(&self) -> Result<Vec<Contact>> {
        self.entity().get_contacts()
    }

    fn as_contact(&mut self) -> Result<Contact> {
        let changes = self.get_changes()?;
        Ok(Contact::new(self.id.clone(), changes))
    }

    fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>> {
        self.entity().get_contact(contact_id)
    }

    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        self.entity().verify_contact(contact)
    }

    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        self.entity().verify_and_add_contact(contact)
    }

    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool> {
        self.entity()
            .verify_and_update_contact(contact_id, change_events)
    }
}

impl SecureChannels for Profile {
    fn create_secure_channel_listener<A: Into<Address> + Send>(
        &mut self,
        address: A,
    ) -> Result<()> {
        self.entity().create_secure_channel_listener(address)
    }

    fn create_secure_channel<R: Into<Route> + Send>(&mut self, route: R) -> Result<Address> {
        self.entity().create_secure_channel(route)
    }
}
