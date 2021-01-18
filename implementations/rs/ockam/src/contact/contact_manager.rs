use crate::contact::contact::{Contact, ContactTags};
use crate::contact::contact_event::ContactEvent;
use crate::contact::error::Error;
use crate::contact::ContactVault;
use crate::profile::profile::Profile;
use ockam_common::error::OckamResult;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type ContactsDb = HashMap<String, (Contact, ContactTags)>;

pub struct ContactManager {
    // TODO: it should be possible to store this to some persistent storage
    contacts: ContactsDb,
}

impl ContactManager {
    pub fn new() -> Self {
        Self {
            contacts: ContactsDb::new(),
        }
    }

    pub fn create_contact_from_profile(
        &self,
        profile: &Profile,
        vault: Arc<Mutex<dyn ContactVault>>,
    ) -> OckamResult<Contact> {
        let events = profile
            .change_events()
            .iter()
            .map(
                |e| ContactEvent::from_profile_event(e).unwrap(), /* FIXME */
            )
            .collect();

        Ok(Contact::new(
            profile.identifier().to_string(),
            events,
            vault,
        ))
    }

    pub fn import_contact(&mut self, contact: Contact, tags: ContactTags) -> OckamResult<String> {
        // TODO: Verify signatures, as well as possible additional proofs

        let identifier = contact.identifier().to_string();
        let db_entry = (contact, tags);
        self.contacts.insert(identifier.clone(), db_entry);

        Ok(identifier)
    }

    pub fn get_contact_public_key(&self, contact_id: &str) -> Option<Vec<u8>> {
        let contact;
        if let Some(c) = self.contacts.get(contact_id) {
            contact = c;
        } else {
            return None;
        }

        let last_event;
        if let Some(e) = contact.0.events().last() {
            last_event = e;
        } else {
            return None;
        }

        last_event.public_key().map(|slice| slice.to_vec())
    }
}
