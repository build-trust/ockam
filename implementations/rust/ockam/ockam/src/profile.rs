use crate::OckamError;
use crate::ProfileChangeType::{CreateKey, RotateKey};
use hashbrown::HashMap;
use ockam_vault_core::{
    HashVault, KeyIdVault, PublicKey, Secret, SecretVault, SignerVault, VerifierVault,
};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

mod identifiers;
pub use identifiers::*;
mod key_attributes;
pub use key_attributes::*;
mod profile_change_proof;
pub use profile_change_proof::*;
mod profile_change_event;
pub use profile_change_event::*;
mod profile_change;
pub use profile_change::*;
mod profile_change_type;
use crate::profile::profile_change_history::ProfileChangeHistory;
pub use profile_change_type::*;

mod profile_change_history;

pub const OCKAM_NO_EVENT: &[u8] = "OCKAM_NO_EVENT".as_bytes();
pub const PROFILE_ROOT_KEY_LABEL: &'static str = "OCKAM_PRK";
pub const OCKAM_PROFILE_VERSION: u8 = 1;
pub const PROFILE_CHANGE_CURRENT_VERSION: u8 = 1;

pub trait ProfileVault: SecretVault + KeyIdVault + HashVault + SignerVault + VerifierVault {}

impl<D> ProfileVault for D where
    D: SecretVault + KeyIdVault + HashVault + SignerVault + VerifierVault
{
}

pub type ProfileEventAttributes = HashMap<String, String>;

/// Profile is an abstraction responsible for keeping, verifying and modifying
/// user's data (mainly - public keys). It is used to create new keys, rotate and revoke them.
/// Public keys together with metadata will be organised into events chain, corresponding
/// secret keys will be saved into the given Vault implementation. Events chain and corresponding
/// secret keys are what fully determines Profile.
///
///
/// # Examples
/// ```
/// use ockam_vault::SoftwareVault;
/// use std::sync::{Mutex, Arc};
/// use ockam::{Profile, KeyAttributes, PROFILE_ROOT_KEY_LABEL, ProfileKeyType, ProfileKeyPurpose};
///
/// fn example() {
///     let vault = SoftwareVault::default();
///     let vault = Arc::new(Mutex::new(vault));
///     let mut profile = Profile::create(None, vault).unwrap();
///
///     let root_key_attributes = KeyAttributes::new(
///         PROFILE_ROOT_KEY_LABEL.to_string(),
///         ProfileKeyType::Root,
///         ProfileKeyPurpose::ProfileUpdate,
///     );
///
///     let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();
///
///     let truck_key_attributes = KeyAttributes::new(
///         "Truck management".to_string(),
///         ProfileKeyType::Issuing,
///         ProfileKeyPurpose::IssueCredentials,
///     );
///
///     profile
///         .create_key(truck_key_attributes.clone(), None)
///         .unwrap();
///
///     let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
///
///     profile.rotate_key(truck_key_attributes.clone(), None).unwrap();
///
///     let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();
///
///     for change_event in profile.change_events().as_ref() {
///         let id = change_event.identifier().to_string_representation();
///         if profile.verify(change_event).is_ok() {
///             println!("{} is valid", id);
///         } else {
///             println!("{} is not valid", id);
///         }
///     }
/// }
/// ```
#[derive(Clone)]
pub struct Profile {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    vault: Arc<Mutex<dyn ProfileVault>>,
}

impl Profile {
    /// Return unique identifier, which equals to sha256 of the root public key
    pub fn identifier(&self) -> &ProfileIdentifier {
        &self.identifier
    }
    /// Return change history chain
    pub fn change_events(&self) -> &[ProfileChangeEvent] {
        self.change_history.as_ref()
    }
}

impl Profile {
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> Self {
        Profile {
            identifier,
            change_history: ProfileChangeHistory::new(change_events),
            vault,
        }
    }
}

impl Profile {
    /// Generate fresh root key and create new [`Profile`]
    pub fn create(
        attributes: Option<ProfileEventAttributes>,
        vault: Arc<Mutex<dyn ProfileVault>>,
    ) -> ockam_core::Result<Self> {
        let mut v = vault.lock().unwrap();
        let prev_id = v.sha256(OCKAM_NO_EVENT)?;
        let prev_id = EventIdentifier::from_hash(prev_id);

        let key_attributes = KeyAttributes::new(
            PROFILE_ROOT_KEY_LABEL.to_string(),
            ProfileKeyType::Root,
            ProfileKeyPurpose::ProfileUpdate,
        );
        let change_event = Self::create_key_event_static(
            prev_id,
            key_attributes.clone(),
            attributes,
            None,
            v.deref_mut(),
        )?;

        let change = ProfileChangeHistory::find_key_change_in_event(&change_event, &key_attributes)
            .ok_or_else(|| OckamError::InvalidInternalState)?;
        let public_key = ProfileChangeHistory::get_change_public_key(&change)?;

        let public_kid = v.compute_key_id_for_public_key(&public_key)?;
        let public_kid = ProfileIdentifier::from_key_id(public_kid);

        let profile = Profile::new(public_kid, vec![change_event], vault.clone());

        Ok(profile)
    }

    /// Create new key
    /// Key is uniquely identified by (label, key_type, key_purpose) triplet in [`KeyAttributes`]
    pub fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()> {
        let root_secret = self.get_root_secret()?;
        let event = self.create_key_event(key_attributes, attributes, Some(&root_secret))?;
        self.apply_no_verification(event)
    }

    /// Rotate existing key
    /// Key is uniquely identified by (label, key_type, key_purpose) triplet in [`KeyAttributes`]
    pub fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> ockam_core::Result<()> {
        let root_key = self.get_root_secret()?;
        let event = self.rotate_key_event(key_attributes, attributes, &root_key)?;
        self.apply_no_verification(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by (label, key_type, key_purpose) triplet in [`KeyAttributes`]
    pub fn get_secret_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<Secret> {
        let event = self.change_history.find_last_key_event(key_attributes)?;
        ProfileChangeHistory::get_secret_key_from_event(
            key_attributes,
            event,
            self.vault.lock().unwrap().deref(),
        )
    }
}

impl Profile {
    fn check_consistency(change_event: &ProfileChangeEvent) -> bool {
        // TODO: check event for consistency: e.g. you cannot rotate the same key twice during one event
        // For only allow one change at a time
        change_event.changes().len() == 1
    }

    fn apply_no_verification(
        &mut self,
        change_event: ProfileChangeEvent,
    ) -> ockam_core::Result<()> {
        if !Self::check_consistency(&change_event) {
            return Err(OckamError::InvalidInternalState.into());
        }

        self.change_history.push_event(change_event);

        Ok(())
    }

    /// Apply new change to the [`Profile`]. Change will be verified
    pub fn apply(&mut self, change_event: ProfileChangeEvent) -> ockam_core::Result<()> {
        self.verify(&change_event)?;

        self.apply_no_verification(change_event)
    }

    /// Verify change relative to current [`Profile`]'s event chain.
    /// WARNING: This function assumes all existing events in chain are verified
    pub fn verify(&self, change_event: &ProfileChangeEvent) -> ockam_core::Result<()> {
        if !Self::check_consistency(&change_event) {
            return Err(OckamError::ConsistencyError.into());
        }

        let changes = change_event.changes();
        let changes_binary = serde_bare::to_vec(&changes).map_err(|_| OckamError::BareError)?;

        let mut vault = self.vault.lock().unwrap();

        let event_id = vault.sha256(&changes_binary)?;
        let event_id = EventIdentifier::from_hash(event_id);

        if &event_id != change_event.identifier() {
            return Err(OckamError::EventIdDoesntMatch.into());
        }

        match change_event.proof() {
            ProfileChangeProof::Signature(s) => match s.stype() {
                SignatureType::RootSign => {
                    let root_public_key = self.get_root_public_key()?;
                    vault.verify(s.data(), root_public_key.as_ref(), event_id.as_ref())?;
                }
            },
        }

        for change in change_event.changes() {
            if !match change.change_type() {
                CreateKey(c) => {
                    // Should have 1 self signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| OckamError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    vault
                        .verify(c.self_signature(), c.data().public_key(), &data_hash)
                        .is_ok()
                }
                RotateKey(c) => {
                    // Should have 1 self signature and 1 prev signature
                    let data_binary =
                        serde_bare::to_vec(c.data()).map_err(|_| OckamError::BareError)?;
                    let data_hash = vault.sha256(data_binary.as_slice())?;

                    if !vault
                        .verify(c.self_signature(), c.data().public_key(), &data_hash)
                        .is_ok()
                    {
                        false;
                    }

                    let prev_key_event = self
                        .change_history
                        .find_key_event_before(&event_id, c.data().key_attributes())?;
                    let prev_key_change = ProfileChangeHistory::find_key_change_in_event(
                        prev_key_event,
                        c.data().key_attributes(),
                    )
                    .ok_or_else(|| OckamError::InvalidInternalState)?;
                    let public_key = ProfileChangeHistory::get_change_public_key(prev_key_change)?;

                    vault
                        .verify(c.prev_signature(), public_key.as_ref(), &data_hash)
                        .is_ok()
                }
            } {
                return Err(OckamError::VerifyFailed.into());
            }
        }

        Ok(())
    }
}

impl Profile {
    pub(crate) fn get_root_secret(&self) -> ockam_core::Result<Secret> {
        let public_key = self.get_root_public_key()?;

        let vault = self.vault.lock().unwrap();

        let key_id = vault.compute_key_id_for_public_key(&public_key)?;
        vault.get_secret_by_key_id(&key_id)
    }

    pub(crate) fn get_root_public_key(&self) -> ockam_core::Result<PublicKey> {
        let root_event;
        if let Some(re) = self.change_events().as_ref().first() {
            root_event = re;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        let root_change;
        if let Some(rc) = root_event.changes().first() {
            root_change = rc;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        let root_create_key_change;
        if let ProfileChangeType::CreateKey(c) = root_change.change_type() {
            root_create_key_change = c;
        } else {
            return Err(OckamError::InvalidInternalState.into());
        }

        Ok(PublicKey::new(
            root_create_key_change.data().public_key().to_vec().into(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_vault::SoftwareVault;

    #[test]
    fn test_new() {
        let vault = SoftwareVault::default();
        let vault = Arc::new(Mutex::new(vault));
        let mut profile = Profile::create(None, vault).unwrap();

        let root_key_attributes = KeyAttributes::new(
            PROFILE_ROOT_KEY_LABEL.to_string(),
            ProfileKeyType::Root,
            ProfileKeyPurpose::ProfileUpdate,
        );

        let _alice_root_secret = profile.get_secret_key(&root_key_attributes).unwrap();

        let truck_key_attributes = KeyAttributes::new(
            "Truck management".to_string(),
            ProfileKeyType::Issuing,
            ProfileKeyPurpose::IssueCredentials,
        );

        profile
            .create_key(truck_key_attributes.clone(), None)
            .unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();

        profile
            .rotate_key(truck_key_attributes.clone(), None)
            .unwrap();

        let _alice_truck_secret = profile.get_secret_key(&truck_key_attributes).unwrap();

        for change_event in profile.change_events().as_ref() {
            let id = change_event.identifier().to_string_representation();
            if profile.verify(change_event).is_ok() {
                println!("{} is valid", id);
            } else {
                println!("{} is not valid", id);
                assert!(false);
            }
        }
    }
}
