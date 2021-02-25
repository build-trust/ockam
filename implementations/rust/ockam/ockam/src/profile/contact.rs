use crate::profile::history::ProfileChangeHistory;
use crate::{
    EventIdentifier, KeyAttributes, OckamError, ProfileChangeEvent, ProfileIdentifier, ProfileVault,
};
use ockam_vault_core::PublicKey;
use serde::{Deserialize, Serialize};

/// Contact is an abstraction responsible for storing user's public data (mainly - public keys).
/// It is designed to share users' public keys in cryptographically verifiable way.
/// Public keys together with metadata are organised into verifiable events chain exactly like [`Profile`].
/// There are two ways to get Contact:
///   1. From another user (in this case Contact will be cryptographically verified)
///   2. Generate one from user's own [`Profile`]
///
/// Public keys from Contact can be used for many purposes, e.g. running key exchange, or signing&encrypting data.
///
/// # Examples
/// ```
/// use ockam_vault::SoftwareVault;
/// use std::sync::{Mutex, Arc};
/// use ockam::{Profile, KeyAttributes, ProfileKeyType, ProfileKeyPurpose};
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
///     let alice_contact = alice_profile.to_contact();
///
///     let public_key = alice_contact.get_public_key(&truck_key_attributes).unwrap();
/// }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
}

impl Contact {
    /// Return unique identifier, which equals to [`Profile`]s identifier
    pub fn identifier(&self) -> &ProfileIdentifier {
        &self.identifier
    }
    /// Return change history chain
    pub fn change_events(&self) -> &[ProfileChangeEvent] {
        self.change_history.as_ref()
    }
}

impl Contact {
    pub fn new(identifier: ProfileIdentifier, change_events: Vec<ProfileChangeEvent>) -> Self {
        Contact {
            identifier,
            change_history: ProfileChangeHistory::new(change_events),
        }
    }
}

impl Contact {
    /// Verify cryptographically whole event chain. Also verify sequence correctness
    pub fn verify(&self, vault: &mut dyn ProfileVault) -> ockam_core::Result<()> {
        ProfileChangeHistory::check_consistency(&[], self.change_events())?;

        for change_event in self.change_events().as_ref() {
            self.change_history.verify_event(change_event, vault)?;
        }

        let root_public_key = self.change_history.get_root_public_key()?;

        let root_key_id = vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if &profile_id != self.identifier() {
            return Err(OckamError::ProfileIdDoesntMatch.into());
        }

        Ok(())
    }

    /// Update [`Contact`] by applying new change events
    pub fn apply(
        &mut self,
        change_events: Vec<ProfileChangeEvent>,
        vault: &mut dyn ProfileVault,
    ) -> ockam_core::Result<()> {
        ProfileChangeHistory::check_consistency(self.change_events(), &change_events)?;

        for event in change_events.iter() {
            self.change_history.verify_event(event, vault)?;
            self.change_history.push_event(event.clone());
        }

        Ok(())
    }
}

impl Contact {
    /// Get root [`PublicKey`]
    pub fn get_root_public_key(&self) -> ockam_core::Result<PublicKey> {
        self.change_history.get_root_public_key()
    }
    /// Get [`PublicKey`]. Key is uniquely identified by (label, key_type, key_purpose) triplet in [`KeyAttributes`]
    pub fn get_public_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<PublicKey> {
        self.change_history.get_public_key(key_attributes)
    }
    /// Get [`EventIdentifier`] of the last known event
    pub fn get_last_event_id(&self) -> ockam_core::Result<EventIdentifier> {
        self.change_history.get_last_event_id()
    }
}
