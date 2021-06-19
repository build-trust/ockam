use crate::authentication::Authentication;
use crate::history::ProfileChangeHistory;
use crate::{
    Contact, ContactsDb, EntityError, EventIdentifier, KeyAttributes, Profile, ProfileAuth,
    ProfileChangeEvent, ProfileChanges, ProfileContacts, ProfileEventAttributes, ProfileIdentifier,
    ProfileIdentity, ProfileSecrets, ProfileVault,
};
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret};

/// Profile implementation
pub struct ProfileImpl<V: ProfileVault> {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: ContactsDb,
    pub(crate) vault: V,
}

impl<V: ProfileVault> ProfileImpl<V> {
    /// Profile constructor
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
        contacts: ContactsDb,
        vault: V,
    ) -> Self {
        Self {
            identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            vault,
        }
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    pub(crate) fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }
    /// Return clone of Vault
    pub fn vault(&self) -> V {
        self.vault.clone()
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    /// Generate fresh [`Profile`] update key and create new [`Profile`] using it
    pub(crate) fn create_internal(
        attributes: Option<ProfileEventAttributes>,
        mut vault: V,
    ) -> Result<Self> {
        let prev_id = vault.sha256(Profile::NO_EVENT)?;
        let prev_id = EventIdentifier::from_hash(prev_id);

        let key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());
        let change_event = Self::create_key_event_static(
            prev_id,
            key_attributes.clone(),
            attributes,
            None,
            &mut vault,
        )?;

        let change = ProfileChangeHistory::find_key_change_in_event(&change_event, &key_attributes)
            .ok_or(EntityError::InvalidInternalState)?;
        let public_key = ProfileChangeHistory::get_change_public_key(change)?;

        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;
        let public_key_id = ProfileIdentifier::from_key_id(public_key_id);

        let profile = Self::new(public_key_id, vec![change_event], Default::default(), vault);

        Ok(profile)
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    pub(crate) fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history.as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }

    pub(crate) fn get_secret_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
        vault: &mut impl ProfileVault,
    ) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_public_key_from_event(key_attributes, event)?;

        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;

        vault.get_secret_by_key_id(&public_key_id)
    }
}

impl<V: ProfileVault> ProfileIdentity for ProfileImpl<V> {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.identifier.clone())
    }
}

impl<V: ProfileVault> ProfileChanges for ProfileImpl<V> {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        Ok(self.change_history.as_ref().to_vec())
    }
    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        let slice = std::slice::from_ref(&change_event);
        ProfileChangeHistory::check_consistency(self.change_history.as_ref(), slice)?;
        self.change_history.push_event(change_event);

        Ok(())
    }
    /// Verify whole event chain of current [`Profile`]
    fn verify(&mut self) -> Result<bool> {
        ProfileChangeHistory::check_consistency(&[], self.change_history().as_ref())?;

        self.change_history
            .verify_all_existing_events(&mut self.vault)?;

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = self.vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if profile_id != self.identifier()? {
            return Err(EntityError::ProfileIdDoesntMatch.into());
        }

        Ok(true)
    }
}

impl<V: ProfileVault> ProfileSecrets for ProfileImpl<V> {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        let event = {
            let root_secret = self.get_root_secret()?;
            self.create_key_event(key_attributes, attributes, Some(&root_secret))?
        };
        self.update_no_verification(event)
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        let event = {
            let root_secret = self.get_root_secret()?;
            self.rotate_key_event(key_attributes, attributes, &root_secret)?
        };
        self.update_no_verification(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        let event = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            key_attributes,
        )?
        .clone();
        Self::get_secret_key_from_event(key_attributes, &event, &mut self.vault)
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        self.change_history.get_public_key(key_attributes)
    }
    fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }
}

impl<V: ProfileVault> ProfileContacts for ProfileImpl<V> {
    fn contacts(&self) -> Result<ContactsDb> {
        Ok(self.contacts.clone())
    }

    fn to_contact(&self) -> Result<Contact> {
        Ok(Contact::new(
            self.identifier.clone(),
            self.change_history.as_ref().to_vec(),
        ))
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        let contact = self.to_contact()?;

        Profile::serialize_contact(&contact)
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        Ok(self.contacts.get(id).cloned())
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        contact.verify(&mut self.vault)?;

        Ok(true)
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        self.verify_contact(&contact)?;

        let _ = self.contacts.insert(contact.identifier().clone(), contact);

        Ok(true)
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        let contact = self
            .contacts
            .get_mut(profile_id)
            .ok_or(EntityError::ContactNotFound)?;

        contact.verify_and_update(change_events, &mut self.vault)?;

        Ok(true)
    }
}

impl<V: ProfileVault> ProfileAuth for ProfileImpl<V> {
    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        let root_secret = self.get_root_secret()?;

        Authentication::generate_proof(channel_state, &root_secret, &mut self.vault)
    }

    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        let contact = self
            .get_contact(responder_contact_id)?
            .ok_or(EntityError::ContactNotFound)?;

        Authentication::verify_proof(
            channel_state,
            &contact.get_profile_update_public_key()?,
            proof,
            &mut self.vault,
        )
    }
}
