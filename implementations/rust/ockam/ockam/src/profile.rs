use crate::OckamError;
use ockam_vault_core::{Hasher, KeyIdVault, PublicKey, Secret, SecretVault, Signer, Verifier};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

mod authentication;
mod contact;
pub use contact::*;
mod identifiers;
pub use identifiers::*;
mod key_attributes;
pub use key_attributes::*;
mod change;
use authentication::Authentication;
pub use change::*;
use history::ProfileChangeHistory;
use ockam_core::lib::HashMap;

pub trait ProfileVault: SecretVault + KeyIdVault + Hasher + Signer + Verifier {}

impl<D> ProfileVault for D where D: SecretVault + KeyIdVault + Hasher + Signer + Verifier {}

pub type ProfileEventAttributes = HashMap<String, String>;
/// Contacts Database
pub type ContactsDb = HashMap<ProfileIdentifier, Contact>;

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
///
/// ```
/// # use ockam_vault::SoftwareVault;
/// # use std::sync::{Mutex, Arc};
/// # use ockam::{Profile, KeyAttributes};
/// let vault = Arc::new(Mutex::new(SoftwareVault::default()));
/// let mut profile = Profile::create(None, vault)?;
///
/// let root_key_attributes = KeyAttributes::new(
///     Profile::PROFILE_UPDATE.to_string(),
/// );
///
/// let _alice_root_secret = profile.get_secret_key(&root_key_attributes)?;
///
/// let truck_key_attributes = KeyAttributes::new(
///     "Truck management".to_string(),
/// );
///
/// profile.create_key(truck_key_attributes.clone(), None)?;
///
/// let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes)?;
///
/// profile.rotate_key(truck_key_attributes.clone(), None)?;
///
/// let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes)?;
///
/// let verified = profile.verify()?;
/// # Ok::<(), ockam_core::Error>(())
/// ```
///
/// Authentication using [`Profile`]. In following example Bob authenticates Alice.
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use ockam_vault::SoftwareVault;
/// # use ockam::Profile;
/// fn alice_main() -> ockam_core::Result<()> {
///     let vault = Arc::new(Mutex::new(SoftwareVault::default()));
///
///     // Alice generates profile
///     let alice = Profile::create(None, vault)?;
///
///     // Key agreement happens here
///     let key_agreement_hash = [0u8; 32];
///
///     // Send this over the network to Bob
///     let contact_alice = alice.serialize_to_contact()?;
///     let proof_alice = alice.generate_authentication_proof(&key_agreement_hash)?;
///
///     Ok(())
/// }
///
/// fn bob_main() -> ockam_core::Result<()> {
///     let vault = Arc::new(Mutex::new(SoftwareVault::default()));
///
///     // Bob generates profile
///     let mut bob = Profile::create(None, vault)?;
///
///     // Key agreement happens here
///     let key_agreement_hash = [0u8; 32];
///
///     // Receive this from Alice over the network
///     # let contact_alice = [0u8; 32];
///     let contact_alice = Profile::deserialize_contact(&contact_alice)?;
///     let alice_id = contact_alice.identifier().clone();
///
///     // Bob adds Alice to contact list
///     bob.verify_and_add_contact(contact_alice)?;
///
///     # let proof_alice = [0u8; 32];
///     // Bob verifies Alice
///     let verified = bob.verify_authentication_proof(&key_agreement_hash, &alice_id, &proof_alice)?;
///
///     Ok(())
/// }
/// ```
///
/// Update [`Profile`] and send changes to other parties. In following example Alice rotates
/// her key and sends corresponding [`Profile`] changes to Bob.
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use ockam_vault::SoftwareVault;
/// # use ockam::Profile;
/// fn alice_main() -> ockam_core::Result<()> {
///     # let vault = Arc::new(Mutex::new(SoftwareVault::default()));
///     # let mut alice = Profile::create(None, vault)?;
///     # let key_agreement_hash = [0u8; 32];
///     # let contact_alice = alice.serialize_to_contact()?;
///     #
///     let index_a = alice.change_events().len();
///     alice.rotate_key(Profile::PROFILE_UPDATE.into(), None)?;
///
///     // Send to Bob
///     let change_events = &alice.change_events()[index_a..];
///     let change_events = Profile::serialize_change_events(change_events)?;
///
///     Ok(())
/// }
///
/// fn bob_main() -> ockam_core::Result<()> {
///     # let vault = Arc::new(Mutex::new(SoftwareVault::default()));
///     # let mut bob = Profile::create(None, vault)?;
///     # let key_agreement_hash = [0u8; 32];
///     # let contact_alice = [0u8; 32];
///     # let contact_alice = Profile::deserialize_contact(&contact_alice)?;
///     # let alice_id = contact_alice.identifier().clone();
///     # bob.verify_and_add_contact(contact_alice)?;
///     // Receive from Alice
///     # let change_events = [0u8; 32];
///     let change_events = Profile::deserialize_change_events(&change_events)?;
///     bob.verify_and_update_contact(&alice_id, change_events)
/// }
/// ```
#[derive(Clone)]
pub struct Profile {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: ContactsDb,
    vault: Arc<Mutex<dyn ProfileVault>>,
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

impl Profile {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    pub fn identifier(&self) -> &ProfileIdentifier {
        &self.identifier
    }
    /// Return change history chain
    pub fn change_events(&self) -> &[ProfileChangeEvent] {
        self.change_history.as_ref()
    }
    /// Return all known to this profile [`Contact`]s
    pub fn contacts(&self) -> &ContactsDb {
        &self.contacts
    }
}

impl Profile {
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
        contacts: ContactsDb,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> Self {
        let profile = Self {
            identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            vault,
        };

        profile
    }
}

impl Profile {
    /// Generate fresh [`Profile`] update key key and create new [`Profile`] using it
    pub fn create(
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> ockam_core::Result<Self> {
        let mut v = vault.lock().unwrap();
        let prev_id = v.sha256(Profile::NO_EVENT)?;
        let prev_id = EventIdentifier::from_hash(prev_id);

        let key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());
        let change_event = Self::create_key_event_static(
            prev_id,
            key_attributes.clone(),
            attributes,
            None,
            v.deref_mut(),
        )?;

        let change = ProfileChangeHistory::find_key_change_in_event(&change_event, &key_attributes)
            .ok_or(OckamError::InvalidInternalState)?;
        let public_key = ProfileChangeHistory::get_change_public_key(&change)?;

        let public_kid = v.compute_key_id_for_public_key(&public_key)?;
        let public_kid = ProfileIdentifier::from_key_id(public_kid);

        let profile = Profile::new(
            public_kid,
            vec![change_event],
            Default::default(),
            vault.clone(),
        );

        Ok(profile)
    }

    /// Create new key. Key is uniquely identified by label in [`KeyAttributes`]
    pub fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()> {
        let event = {
            let mut vault = self.vault.lock().unwrap();
            let root_secret = self.get_root_secret(vault.deref_mut())?;
            self.create_key_event(
                key_attributes,
                attributes,
                Some(&root_secret),
                vault.deref_mut(),
            )?
        };
        self.update_no_verification(event)
    }

    /// Rotate existing key. Key is uniquely identified by label in [`KeyAttributes`]
    pub fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()> {
        let event = {
            let mut vault = self.vault.lock().unwrap();
            let root_secret = self.get_root_secret(vault.deref_mut())?;
            self.rotate_key_event(key_attributes, attributes, &root_secret, vault.deref_mut())?
        };
        self.update_no_verification(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    pub fn get_secret_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<Secret> {
        let event =
            ProfileChangeHistory::find_last_key_event(self.change_events(), key_attributes)?;
        Self::get_secret_key_from_event(
            key_attributes,
            event,
            self.vault.lock().unwrap().deref_mut(),
        )
    }

    /// Get [`PublicKey`]. Key is uniquely identified by label in [`KeyAttributes`]
    pub fn get_public_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<PublicKey> {
        self.change_history.get_public_key(key_attributes)
    }
}

impl Profile {
    fn update_no_verification(
        &mut self,
        change_event: ProfileChangeEvent,
    ) -> ockam_core::Result<()> {
        let slice = std::slice::from_ref(&change_event);
        ProfileChangeHistory::check_consistency(self.change_events(), &slice)?;
        self.change_history.push_event(change_event);

        Ok(())
    }

    /// Verify whole event chain of current [`Profile`]
    pub fn verify(&self) -> ockam_core::Result<()> {
        ProfileChangeHistory::check_consistency(&[], self.change_events())?;

        let mut vault = self.vault.lock().unwrap();

        self.change_history
            .verify_all_existing_events(vault.deref_mut())?;

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if &profile_id != self.identifier() {
            return Err(OckamError::ProfileIdDoesntMatch.into());
        }

        Ok(())
    }
}

impl Profile {
    pub(crate) fn get_root_secret(
        &self,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<Secret> {
        let public_key =
            ProfileChangeHistory::get_current_profile_update_public_key(self.change_events())?;

        let key_id = vault.compute_key_id_for_public_key(&public_key)?;
        vault.get_secret_by_key_id(&key_id)
    }

    pub(crate) fn get_secret_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<Secret> {
        let public_key = ProfileChangeHistory::get_public_key_from_event(key_attributes, event)?;

        let public_kid = vault.compute_key_id_for_public_key(&public_key)?;

        vault.get_secret_by_key_id(&public_kid)
    }
}

// Contacts
impl Profile {
    /// Convert [`Profile`] to [`Contact`]
    pub fn to_contact(&self) -> Contact {
        Contact::new(
            self.identifier.clone(),
            self.change_history.as_ref().to_vec(),
        )
    }

    /// Serialize [`Profile`] to [`Contact`] in binary form for storing/transferring over the network
    pub fn serialize_to_contact(&self) -> ockam_core::Result<Vec<u8>> {
        let contact = self.to_contact();

        Profile::serialize_contact(&contact)
    }

    /// Serialize [`Contact`] in binary form for storing/transferring over the network
    pub fn serialize_contact(contact: &Contact) -> ockam_core::Result<Vec<u8>> {
        serde_bare::to_vec(&contact).map_err(|_| OckamError::BareError.into())
    }

    /// Deserialize [`Contact`] from binary form
    pub fn deserialize_contact(contact: &[u8]) -> ockam_core::Result<Contact> {
        let contact: Contact =
            serde_bare::from_slice(contact).map_err(|_| OckamError::BareError)?;

        Ok(contact)
    }

    /// Serialize [`ProfileChangeEvent`]s to binary form for storing/transferring over the network
    pub fn serialize_change_events(
        change_events: &[ProfileChangeEvent],
    ) -> ockam_core::Result<Vec<u8>> {
        serde_bare::to_vec(&change_events).map_err(|_| OckamError::BareError.into())
    }

    /// Deserialize [`ProfileChangeEvent`]s from binary form
    pub fn deserialize_change_events(
        change_events: &[u8],
    ) -> ockam_core::Result<Vec<ProfileChangeEvent>> {
        let change_events: Vec<ProfileChangeEvent> =
            serde_bare::from_slice(change_events).map_err(|_| OckamError::BareError)?;

        Ok(change_events)
    }

    /// Return [`Contact`] with given [`ProfileIdentifier`]
    pub fn get_contact(&self, id: &ProfileIdentifier) -> Option<&Contact> {
        self.contacts.get(id)
    }

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    pub fn verify_contact(&self, contact: &Contact) -> ockam_core::Result<()> {
        let mut vault = self.vault.lock().unwrap();
        contact.verify(vault.deref_mut())
    }

    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    pub fn verify_and_add_contact(&mut self, contact: Contact) -> ockam_core::Result<()> {
        self.verify_contact(&contact)?;

        let _ = self.contacts.insert(contact.identifier().clone(), contact);

        Ok(())
    }

    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    pub fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> ockam_core::Result<()> {
        let contact = self
            .contacts
            .get_mut(profile_id)
            .ok_or(OckamError::ContactNotFound)?;

        let mut vault = self.vault.lock().unwrap();
        contact.verify_and_update(change_events, vault.deref_mut())
    }
}

// Authentication
impl Profile {
    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    pub fn generate_authentication_proof(
        &self,
        channel_state: &[u8],
    ) -> ockam_core::Result<Vec<u8>> {
        let mut vault = self.vault.lock().unwrap();

        let root_secret = self.get_root_secret(vault.deref_mut())?;

        Authentication::generate_proof(channel_state, &root_secret, vault.deref_mut())
    }

    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    pub fn verify_authentication_proof(
        &self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> ockam_core::Result<bool> {
        let contact = self
            .get_contact(responder_contact_id)
            .ok_or(OckamError::ContactNotFound)?;

        let mut vault = self.vault.lock().unwrap();

        Authentication::verify_proof(
            channel_state,
            &contact.get_profile_update_public_key()?,
            proof,
            vault.deref_mut(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_vault::SoftwareVault;

    #[test]
    fn test_new() {
        let vault = Arc::new(Mutex::new(SoftwareVault::default()));
        let mut profile = Profile::create(None, vault).unwrap();

        profile.verify().unwrap();

        let root_key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());

        let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();
        let _alice_root_public_key = profile.get_public_key(&root_key_attributes).unwrap();

        let truck_key_attributes = KeyAttributes::new("Truck management".to_string());

        profile
            .create_key(truck_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
        let _alice_truck_public_key = profile.get_public_key(&truck_key_attributes).unwrap();

        profile
            .rotate_key(truck_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
        let _alice_truck_public_key = profile.get_public_key(&truck_key_attributes).unwrap();

        profile
            .rotate_key(root_key_attributes.clone(), None)
            .unwrap();

        profile.verify().unwrap();

        let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();
        let _alice_root_public_key = profile.get_public_key(&root_key_attributes).unwrap();
    }

    #[test]
    fn test_update() {
        let vault = Arc::new(Mutex::new(SoftwareVault::default()));
        let mut alice = Profile::create(None, vault.clone()).unwrap();

        let mut bob = Profile::create(None, vault).unwrap();

        // Receive this from Alice over the network
        let contact_alice = alice.serialize_to_contact().unwrap();
        let contact_alice = Profile::deserialize_contact(&contact_alice).unwrap();
        let alice_id = contact_alice.identifier().clone();
        // Bob adds Alice to contact list
        bob.verify_and_add_contact(contact_alice).unwrap();

        alice
            .rotate_key(Profile::PROFILE_UPDATE.into(), None)
            .unwrap();

        let index_a = alice.change_events().len();
        let change_events = &alice.change_events()[index_a..];
        let change_events = Profile::serialize_change_events(change_events).unwrap();

        // Receive from Alice
        let change_events = Profile::deserialize_change_events(&change_events).unwrap();
        bob.verify_and_update_contact(&alice_id, change_events)
            .unwrap();
    }
}
