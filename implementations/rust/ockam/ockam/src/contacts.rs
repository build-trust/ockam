mod contact;
use crate::{OckamError, ProfileChangeEvent, ProfileIdentifier, ProfileVault};
pub use contact::*;
use hashbrown::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

type ContactsDb = HashMap<ProfileIdentifier, Contact>;

/// Contacts is an in-memory storage for a list of user's [`Contact`]s.
///
/// # Examples
/// ```
/// use ockam_vault::SoftwareVault;
/// use std::sync::{Mutex, Arc};
/// use ockam::{Profile, KeyAttributes, ProfileKeyType, ProfileKeyPurpose, Contacts};
///
/// fn example() {
///     let vault = SoftwareVault::default();
///     let vault = Arc::new(Mutex::new(vault));
///     let mut alice_profile = Profile::create(None, vault.clone()).unwrap();
///
///     let truck_key_attributes = KeyAttributes::new(
///         "Truck management".to_string(),
///         ProfileKeyType::Issuing,
///         ProfileKeyPurpose::IssueCredentials,
///     );
///
///     alice_profile
///         .create_key(truck_key_attributes.clone(), None)
///         .unwrap();
///
///     let alice_id = alice_profile.identifier().clone();
///     let alice_contact = alice_profile.to_contact();
///
///     let mut contacts = Contacts::new(Default::default(), vault);
///
///     contacts.add_contact(alice_contact).unwrap();
///
///     let alice_contact = contacts.get_contact(&alice_id).unwrap();
///     let last_known_event = alice_contact.get_last_event_id().unwrap();
///
///     let public_key = alice_contact.get_public_key(&truck_key_attributes).unwrap();
///
///     alice_profile.rotate_key(truck_key_attributes.clone(), None).unwrap();
///
///     let new_changes = alice_profile.get_events_after(&last_known_event).unwrap();
///
///     contacts.apply_to_contact(&alice_id, new_changes.to_vec()).unwrap();
///
///     let alice_contact = contacts.get_contact(&alice_id).unwrap();
///     let new_public_key = alice_contact.get_public_key(&truck_key_attributes).unwrap();
///
///     contacts.verify(&alice_id).unwrap();
/// }
/// ```
pub struct Contacts {
    // TODO: it should be possible to store this to some persistent storage
    contacts: ContactsDb,
    vault: Arc<Mutex<dyn ProfileVault>>,
}

impl Contacts {
    pub fn new(contacts: ContactsDb, vault: Arc<Mutex<dyn ProfileVault>>) -> Self {
        Contacts { contacts, vault }
    }
}

impl Contacts {
    /// Return [`Contact`]
    pub fn get_contact(&self, id: &ProfileIdentifier) -> Option<&Contact> {
        self.contacts.get(id)
    }

    /// Add new [`Contact`]
    pub fn add_contact(&mut self, contact: Contact) -> ockam_core::Result<()> {
        let mut vault = self.vault.lock().unwrap();
        contact.verify(vault.deref_mut())?;

        let _ = self.contacts.insert(contact.identifier().clone(), contact);

        Ok(())
    }

    /// Update [`Contact`] by applying new change events
    pub fn apply_to_contact(
        &mut self,
        id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> ockam_core::Result<()> {
        let contact;
        if let Some(c) = self.contacts.get_mut(id) {
            contact = c;
        } else {
            return Err(OckamError::ContactNotFound.into());
        }

        let mut vault = self.vault.lock().unwrap();

        contact.apply(change_events, vault.deref_mut())
    }

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    pub fn verify(&self, id: &ProfileIdentifier) -> ockam_core::Result<()> {
        let contact;
        if let Some(c) = self.contacts.get(id) {
            contact = c;
        } else {
            return Err(OckamError::ContactNotFound.into());
        }

        let mut vault = self.vault.lock().unwrap();

        contact.verify(vault.deref_mut())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{KeyAttributes, Profile, ProfileKeyPurpose, ProfileKeyType};
    use ockam_vault::SoftwareVault;

    #[test]
    fn test_add_contact() {
        let vault = SoftwareVault::default();
        let vault = Arc::new(Mutex::new(vault));
        let mut alice_profile = Profile::create(None, vault.clone()).unwrap();

        let truck_key_attributes = KeyAttributes::new(
            "Truck management".to_string(),
            ProfileKeyType::Issuing,
            ProfileKeyPurpose::IssueCredentials,
        );

        alice_profile
            .create_key(truck_key_attributes.clone(), None)
            .unwrap();

        let alice_id = alice_profile.identifier();
        let alice_contact = alice_profile.to_contact();

        let alice_contact = serde_bare::to_vec(&alice_contact).unwrap();
        let alice_contact: Contact = serde_bare::from_slice(alice_contact.as_slice()).unwrap();

        let mut contacts = Contacts::new(Default::default(), vault);

        contacts.add_contact(alice_contact).unwrap();

        let _public_key = contacts
            .get_contact(alice_id)
            .unwrap()
            .get_public_key(&truck_key_attributes)
            .unwrap();

        contacts.verify(&alice_id).unwrap();
    }

    #[test]
    fn test_update_contact() {
        let vault = SoftwareVault::default();
        let vault = Arc::new(Mutex::new(vault));
        let mut alice_profile = Profile::create(None, vault.clone()).unwrap();

        let truck_key_attributes = KeyAttributes::new(
            "Truck management".to_string(),
            ProfileKeyType::Issuing,
            ProfileKeyPurpose::IssueCredentials,
        );

        alice_profile
            .create_key(truck_key_attributes.clone(), None)
            .unwrap();

        let alice_id = alice_profile.identifier().clone();
        let alice_contact = alice_profile.to_contact();

        let mut contacts = Contacts::new(Default::default(), vault);

        contacts.add_contact(alice_contact).unwrap();

        let alice_contact = contacts.get_contact(&alice_id).unwrap();
        let last_known_event = alice_contact.get_last_event_id().unwrap();

        let public_key = alice_contact.get_public_key(&truck_key_attributes).unwrap();

        alice_profile
            .rotate_key(truck_key_attributes.clone(), None)
            .unwrap();

        let new_changes = alice_profile.get_events_after(&last_known_event).unwrap();

        contacts
            .apply_to_contact(&alice_id, new_changes.to_vec())
            .unwrap();

        let alice_contact = contacts.get_contact(&alice_id).unwrap();
        let new_public_key = alice_contact.get_public_key(&truck_key_attributes).unwrap();

        assert_ne!(public_key, new_public_key);

        contacts.verify(&alice_id).unwrap();
    }
}
