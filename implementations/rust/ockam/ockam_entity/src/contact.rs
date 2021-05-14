use crate::history::ProfileChangeHistory;
use crate::{
    EntityError, EventIdentifier, KeyAttributes, ProfileChangeEvent, ProfileIdentifier,
    ProfileVault,
};
use ockam_vault_core::PublicKey;
use serde::{Deserialize, Serialize};

/// Contact is an abstraction responsible for storing user's public data (mainly - public keys).
/// It is designed to share users' public keys in cryptographically verifiable way.
/// Public keys together with metadata are organised into verifiable events chain exactly like [`crate::Profile`].
/// There are two ways to get Contact:
///   1. From another user (in this case Contact will be cryptographically verified)
///   2. Generate one from user's own [`crate::Profile`]
///
/// Public keys from Contact can be used for many purposes, e.g. running key exchange, or signing&encrypting data.
///
/// # Examples
///
/// Creating [`Contact`] from [`crate::Profile`]
///
/// ```
/// # use ockam_vault::SoftwareVault;
/// # use ockam_vault_sync_core::Vault;
/// # use ockam_entity::{Profile, KeyAttributes, ProfileSecrets, ProfileContacts};
/// #
/// # fn main() -> ockam_core::Result<()> {
/// # let (mut ctx, mut executor) = ockam_node::start_node();
/// # executor.execute(async move {
/// let vault = Vault::create(&ctx)?;
/// let mut alice = Profile::create(&ctx, &vault).await?;
///
/// let truck_key_attributes = KeyAttributes::new(
///     "Truck management".to_string(),
/// );
///
/// alice.create_key(truck_key_attributes.clone(), None)?;
///
/// let alice_contact = alice.to_contact();
///
/// let public_key = alice.get_public_key(&truck_key_attributes)?;
/// # ctx.stop().await.unwrap();
/// # Ok::<(), ockam_core::Error>(())
/// # }).unwrap();
/// # Ok(())
/// # }
/// ```
///
/// Sending Contact over the network
///
/// ```
/// # use ockam_vault::SoftwareVault;
/// # use ockam_entity::{Profile, KeyAttributes, ProfileSecrets, ProfileContacts};
/// #
/// # fn main() -> ockam_core::Result<()> {
/// # use ockam_vault_sync_core::Vault;
/// # let (mut ctx, mut executor) = ockam_node::start_node();
/// # executor.execute(async move {
/// # let vault = Vault::create(&ctx)?;
/// # let mut alice = Profile::create(&ctx, &vault).await?;
/// #
/// # let truck_key_attributes = KeyAttributes::new(
/// #     "Truck management".to_string(),
/// # );
/// #
/// # alice.create_key(truck_key_attributes.clone(), None)?;
/// #
/// // Send this over the network
/// let alice_contact_binary = alice.serialize_to_contact()?;
/// # ctx.stop().await.unwrap();
/// # Ok::<(), ockam_core::Error>(())
/// # }).unwrap();
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
}

impl Contact {
    /// Return unique identifier, which equals to [`crate::Profile`]'s identifier
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
    pub fn verify(&self, vault: &mut impl ProfileVault) -> ockam_core::Result<()> {
        ProfileChangeHistory::check_consistency(&[], self.change_events())?;

        self.change_history.verify_all_existing_events(vault)?;

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if &profile_id != self.identifier() {
            return Err(EntityError::ProfileIdDoesntMatch.into());
        }

        Ok(())
    }

    /// Update [`Contact`] by using new change events
    pub fn verify_and_update(
        &mut self,
        change_events: Vec<ProfileChangeEvent>,
        vault: &mut impl ProfileVault,
    ) -> ockam_core::Result<()> {
        ProfileChangeHistory::check_consistency(self.change_events(), &change_events)?;

        for event in change_events.iter() {
            ProfileChangeHistory::verify_event(self.change_events(), event, vault)?;
            self.change_history.push_event(event.clone());
        }

        Ok(())
    }
}

impl Contact {
    /// Get [`crate::Profile`] Update [`PublicKey`]
    pub fn get_profile_update_public_key(&self) -> ockam_core::Result<PublicKey> {
        ProfileChangeHistory::get_current_profile_update_public_key(self.change_events())
    }
    /// Get [`PublicKey`]. Key is uniquely identified by label in [`KeyAttributes`]
    pub fn get_public_key(&self, key_attributes: &KeyAttributes) -> ockam_core::Result<PublicKey> {
        self.change_history.get_public_key(key_attributes)
    }
    /// Get [`EventIdentifier`] of the last known event
    pub fn get_last_event_id(&self) -> ockam_core::Result<EventIdentifier> {
        self.change_history.get_last_event_id()
    }
}
