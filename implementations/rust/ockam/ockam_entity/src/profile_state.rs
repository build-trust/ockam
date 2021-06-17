use ockam_core::{allow, deny, Result};
use ockam_vault::{KeyIdVault, PublicKey, Secret, SecretAttributes};
use ockam_vault_sync_core::VaultSync;

use crate::change_history::ProfileChangeHistory;
use crate::{
    authentication::Authentication,
    profile::Profile,
    Changes, Contact, Contacts, EntityError,
    EntityError::{ContactVerificationFailed, InvalidInternalState},
    EventIdentifier, Identity, KeyAttributes, MetaKeyAttributes, ProfileChangeEvent,
    ProfileEventAttributes, ProfileIdentifier, ProfileVault, Proof,
};
use ockam_vault_core::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};

/// Profile implementation
#[derive(Clone)]
pub struct ProfileState {
    id: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: Contacts,
    vault: VaultSync,
    key_attribs: KeyAttributes,
}

impl ProfileState {
    /// Profile constructor
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Changes,
        contacts: Contacts,
        vault: VaultSync,
        key_attribs: KeyAttributes,
    ) -> Self {
        Self {
            id: identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            vault,
            key_attribs,
        }
    }

    pub(crate) fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }
    /// Return clone of Vault
    pub fn vault(&self) -> VaultSync {
        self.vault.clone()
    }

    /// Create ProfileState
    pub(crate) fn create(mut vault: VaultSync) -> Result<Self> {
        let initial_event_id = EventIdentifier::initial(vault.clone());

        let key_attribs = KeyAttributes::with_attributes(
            Profile::PROFILE_UPDATE.to_string(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        );

        let create_key_event = Self::create_key_static(
            initial_event_id,
            key_attribs.clone(),
            ProfileEventAttributes::new(),
            None,
            &mut vault,
        )?;

        let create_key_change =
            ProfileChangeHistory::find_key_change_in_event(&create_key_event, &key_attribs)
                .ok_or(InvalidInternalState)?;

        let public_key = ProfileChangeHistory::get_change_public_key(&create_key_change)?;
        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;
        let public_key_id = ProfileIdentifier::from_key_id(public_key_id);

        let profile = Self::new(
            public_key_id,
            vec![create_key_event],
            Default::default(),
            vault,
            key_attribs,
        );

        Ok(profile)
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

    pub fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }
}

impl Identity for ProfileState {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.id.clone())
    }

    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()> {
        self.key_attribs =
            KeyAttributes::with_attributes(label.into(), self.key_attribs.meta().clone());

        let event = { self.create_key(self.key_attribs.clone(), ProfileEventAttributes::new())? };
        self.add_change(event)
    }

    fn rotate_key(&mut self) -> Result<()> {
        let event = { self.rotate_key(self.key_attribs.clone(), ProfileEventAttributes::new())? };
        self.add_change(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&self) -> Result<Secret> {
        let event = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            &self.key_attribs,
        )?
        .clone();
        Self::get_secret_key_from_event(&self.key_attribs, &event, &mut self.vault.clone())
    }

    fn get_public_key(&self) -> Result<PublicKey> {
        self.change_history.get_public_key(&self.key_attribs)
    }

    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn create_proof<S: AsRef<[u8]>>(&mut self, channel_state: S) -> Result<Proof> {
        let root_secret = self.get_root_secret()?;

        Authentication::generate_proof(channel_state.as_ref(), &root_secret, &mut self.vault)
    }
    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn verify_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        channel_state: S,
        responder_contact_id: &ProfileIdentifier,
        proof: P,
    ) -> Result<bool> {
        let contact = self
            .get_contact(responder_contact_id)?
            .ok_or(EntityError::ContactNotFound)?;

        Authentication::verify_proof(
            channel_state.as_ref(),
            &contact.get_profile_update_public_key()?,
            proof.as_ref(),
            &mut self.vault,
        )
    }

    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        let slice = std::slice::from_ref(&change_event);
        if ProfileChangeHistory::check_consistency(self.change_history.as_ref(), &slice) {
            self.change_history.push_event(change_event);
        }
        Ok(())
    }

    fn get_changes(&self) -> Result<Changes> {
        Ok(self.change_history.as_ref().to_vec())
    }

    /// Verify whole event chain of current [`Profile`]
    fn verify_changes(&mut self) -> Result<bool> {
        if !ProfileChangeHistory::check_consistency(&[], self.change_history().as_ref()) {
            return deny();
        }

        if !self
            .change_history
            .verify_all_existing_events(&mut self.vault)?
        {
            return deny();
        }

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = self.vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if profile_id != self.identifier()? {
            return Err(EntityError::ProfileIdDoesntMatch.into());
        }

        allow()
    }

    fn get_contacts(&self) -> Result<Vec<Contact>> {
        Ok(self.contacts.values().cloned().collect())
    }

    fn as_contact(&mut self) -> Result<Contact> {
        Ok(Contact::new(
            self.id.clone(),
            self.change_history.as_ref().to_vec(),
        ))
    }

    fn get_contact(&mut self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        Ok(self.contacts.get(id).cloned())
    }

    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        contact.verify(&mut self.vault)?;

        allow()
    }

    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        if !self.verify_contact(contact.clone())? {
            return Err(ContactVerificationFailed.into());
        }

        self.contacts.insert(contact.identifier().clone(), contact);

        allow()
    }

    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool> {
        let contact = self
            .contacts
            .get_mut(&contact_id)
            .ok_or(EntityError::ContactNotFound)
            .expect("contact not found");

        Ok(contact.verify_and_update(change_events, &mut self.vault)?)
    }
}
