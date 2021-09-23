use crate::change_history::ProfileChangeHistory;
use crate::{
    authentication::Authentication,
    profile::Profile,
    AuthenticationProof, Changes, Contact, Contacts, EntityError,
    EntityError::{ContactVerificationFailed, InvalidInternalState},
    EventIdentifier, Identity, KeyAttributes, Lease, MetaKeyAttributes, ProfileChangeEvent,
    ProfileEventAttributes, ProfileIdentifier, ProfileVault, TTL,
};
use cfg_if::cfg_if;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::rand::{thread_rng, CryptoRng, RngCore};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::traits::AsyncClone;
use ockam_core::{allow, deny, Result, Route};
use ockam_vault::{KeyIdVault, PublicKey, Secret, SecretAttributes};
use ockam_vault_core::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use ockam_vault_sync_core::VaultSync;

cfg_if! {
    if #[cfg(feature = "credentials")] {
        use signature_core::message::Message;
        use crate::credential::EntityCredential;
    }
}

/// Profile implementation
#[derive(Clone)]
pub struct ProfileState {
    id: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: Contacts,
    pub(crate) vault: VaultSync,
    #[cfg(feature = "credentials")]
    pub(crate) rand_msg: Message,
    #[cfg(feature = "credentials")]
    pub(crate) credentials: Vec<EntityCredential>,
    lease: Option<Lease>,
}

impl ProfileState {
    /// Profile constructor
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Changes,
        contacts: Contacts,
        vault: VaultSync,
        rng: impl RngCore + CryptoRng + Clone,
    ) -> Self {
        // Avoid warning
        let _ = rng;
        Self {
            id: identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            vault,
            #[cfg(feature = "credentials")]
            rand_msg: Message::random(rng),
            #[cfg(feature = "credentials")]
            credentials: vec![],
            lease: None,
        }
    }

    pub(crate) fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }

    /// Return clone of Vault
    pub fn vault(&self) -> VaultSync {
        self.vault.clone()
    }

    /// Return clone of Vault
    pub async fn async_vault(&self) -> VaultSync {
        self.vault.async_clone().await
    }

    /// Create ProfileState
    pub(crate) async fn async_create(mut vault: VaultSync) -> Result<Self> {
        let hasher = vault.async_start_another().await.unwrap();
        let initial_event_id = EventIdentifier::async_initial(hasher).await;

        let key_attribs = KeyAttributes::with_attributes(
            Profile::PROFILE_UPDATE.to_string(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        );

        let create_key_event = Self::async_create_key_static(
            initial_event_id,
            key_attribs.clone(),
            ProfileEventAttributes::new(),
            None,
            &mut vault,
        )
        .await?;

        let create_key_change =
            ProfileChangeHistory::find_key_change_in_event(&create_key_event, &key_attribs)
                .ok_or(InvalidInternalState)?;

        let public_key = ProfileChangeHistory::get_change_public_key(&create_key_change)?;
        let public_key_id = vault
            .async_compute_key_id_for_public_key(&public_key)
            .await?;
        let public_key_id = ProfileIdentifier::from_key_id(public_key_id);

        let profile = Self::new(
            public_key_id,
            vec![create_key_event],
            Default::default(),
            vault,
            thread_rng(),
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

    pub async fn async_get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self
            .vault
            .async_compute_key_id_for_public_key(&public_key)
            .await?;
        self.vault.async_get_secret_by_key_id(&key_id).await
    }

    pub fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }

    pub fn has_lease(&self) -> bool {
        self.lease.is_some()
    }

    pub fn lease(&self) -> Option<&Lease> {
        self.lease.as_ref()
    }
}

#[async_trait]
impl AsyncClone for ProfileState {
    async fn async_clone(&self) -> ProfileState {
        self.clone()
    }
}

#[async_trait]
impl Identity for ProfileState {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.id.clone())
    }

    async fn async_identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.id.clone())
    }

    fn create_key<S: Into<String> + Send + 'static>(&mut self, label: S) -> Result<()> {
        let key_attribs = KeyAttributes::new(label.into());

        let event = { self.sync_create_key(key_attribs, ProfileEventAttributes::new())? };
        self.add_change(event)
    }

    async fn async_create_key<S: Into<String> + Send + 'static>(&mut self, label: S) -> Result<()> {
        let key_attribs = KeyAttributes::new(label.into());

        let event = {
            self.async_create_key(key_attribs, ProfileEventAttributes::new())
                .await?
        };
        self.add_change(event)
    }

    fn rotate_profile_key(&mut self) -> Result<()> {
        let event = {
            self.rotate_key(
                KeyAttributes::new(Profile::PROFILE_UPDATE.to_string()),
                ProfileEventAttributes::new(),
            )?
        };
        self.add_change(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_profile_secret_key(&self) -> Result<Secret> {
        self.get_secret_key(Profile::PROFILE_UPDATE)
    }

    fn get_secret_key<S: Into<String>>(&self, label: S) -> Result<Secret> {
        let key_attributes = KeyAttributes::new(label.into());
        let event = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            &key_attributes,
        )?
        .clone();
        Self::get_secret_key_from_event(&key_attributes, &event, &mut self.vault.clone())
    }

    fn get_profile_public_key(&self) -> Result<PublicKey> {
        self.get_public_key(Profile::PROFILE_UPDATE)
    }

    fn get_public_key<S: Into<String>>(&self, label: S) -> Result<PublicKey> {
        self.change_history
            .get_public_key(&KeyAttributes::new(label.into()))
    }

    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn create_auth_proof<S: AsRef<[u8]>>(
        &mut self,
        channel_state: S,
    ) -> Result<AuthenticationProof> {
        let root_secret = self.get_root_secret()?;

        Authentication::generate_proof(channel_state.as_ref(), &root_secret, &mut self.vault)
    }

    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    async fn async_create_auth_proof<S: AsRef<[u8]> + Send + Sync>(
        &mut self,
        channel_state: S,
    ) -> Result<AuthenticationProof> {
        let root_secret = self.async_get_root_secret().await?;

        Authentication::async_generate_proof(channel_state.as_ref(), &root_secret, &mut self.vault)
            .await
    }

    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn verify_auth_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
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

    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    async fn async_verify_auth_proof<S: AsRef<[u8]> + Send + Sync, P: AsRef<[u8]> + Send + Sync>(
        &mut self,
        channel_state: S,
        responder_contact_id: &ProfileIdentifier,
        proof: P,
    ) -> Result<bool> {
        let contact = self
            .async_get_contact(responder_contact_id)
            .await?
            .ok_or(EntityError::ContactNotFound)?;

        Authentication::async_verify_proof(
            channel_state.as_ref(),
            &contact.get_profile_update_public_key()?,
            proof.as_ref(),
            &mut self.vault,
        )
        .await
    }

    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        let slice = core::slice::from_ref(&change_event);
        if ProfileChangeHistory::check_consistency(self.change_history.as_ref(), &slice) {
            self.change_history.push_event(change_event);
        }
        Ok(())
    }

    fn get_changes(&self) -> Result<Changes> {
        Ok(self.change_history.as_ref().to_vec())
    }

    async fn async_get_changes(&self) -> Result<Changes> {
        self.get_changes()
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

    async fn async_as_contact(&mut self) -> Result<Contact> {
        self.as_contact()
    }

    fn get_contact(&mut self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        Ok(self.contacts.get(id).cloned())
    }

    async fn async_get_contact(&mut self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        self.get_contact(id)
    }

    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        contact.verify(&mut self.vault)?;

        allow()
    }

    async fn async_verify_contact<C: Into<Contact> + Send>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        contact.async_verify(&mut self.vault).await?;

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

    async fn async_verify_and_add_contact<C: Into<Contact> + Send>(
        &mut self,
        contact: C,
    ) -> Result<bool> {
        let contact = contact.into();
        if !self.async_verify_contact(contact.clone()).await? {
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

    fn get_lease(
        &self,
        _lease_manager_route: &Route,
        _org_id: impl ToString,
        _bucket: impl ToString,
        _ttl: TTL,
    ) -> Result<Lease> {
        if let Some(lease) = self.lease.clone() {
            Ok(lease)
        } else {
            Err(InvalidInternalState.into())
        }
    }

    fn revoke_lease(&mut self, _lease_manager_route: &Route, lease: Lease) -> Result<()> {
        if let Some(existing_lease) = &self.lease {
            if existing_lease == &lease {
                self.lease = None
            }
        }
        Ok(())
    }
}
