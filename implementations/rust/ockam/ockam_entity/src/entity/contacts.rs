//! Entity contacts
//!
use crate::EntityError::ProfileNotFound;
use crate::{
    Contact, ContactsDb, Entity, ProfileChangeEvent, ProfileContacts, ProfileIdentifier,
    ProfileTrait,
};
use ockam_core::Result;

impl<P: ProfileTrait> ProfileContacts for Entity<P> {
    fn contacts(&self) -> Result<ContactsDb> {
        if let Some(profile) = self.default_profile() {
            profile.contacts()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn to_contact(&self) -> Result<Contact> {
        if let Some(profile) = self.default_profile() {
            profile.to_contact()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        if let Some(profile) = self.default_profile() {
            profile.serialize_to_contact()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        if let Some(profile) = self.default_profile() {
            profile.get_contact(id)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.verify_contact(contact)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.verify_and_add_contact(contact)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        if let Some(profile) = self.profiles.get_mut(&self.default_profile_identifier) {
            profile.verify_and_update_contact(profile_id, change_events)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}
