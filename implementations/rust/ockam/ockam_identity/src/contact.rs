/// Contact is an abstraction responsible for storing user's public data (mainly - public keys).
use serde::{Deserialize, Serialize};

use ockam_vault::PublicKey;

use crate::change_history::IdentityChangeHistory;
use crate::{EventIdentifier, IdentityChangeEvent, IdentityIdentifier, IdentityVault};

use ockam_core::compat::vec::Vec;
use ockam_core::error::{allow, deny, Result};

/// Contact is an abstraction responsible for storing user's public data (mainly - public keys).
/// It is designed to share users' public keys in cryptographically verifiable way.
/// Public keys together with metadata are organised into verifiable events chain exactly like [`crate::Identity`].
/// There are two ways to get Contact:
///   1. From another user (in this case Contact will be cryptographically verified)
///   2. Generate one from user's own [`Identity`](crate::Identity)
///
/// Public keys from Contact can be used for many purposes, e.g. running key exchange, or signing&encrypting data.
///
/// # Examples
///
/// Creating [`Contact`] from [`Identity`](crate::Identity)
///
/// TODO
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contact {
    identifier: IdentityIdentifier,
    change_history: IdentityChangeHistory,
}

impl Contact {
    /// Return unique identifier, which equals to [`Identity`](crate::Identity)'s identifier
    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.identifier
    }
    /// Return change history chain
    pub fn change_events(&self) -> &[IdentityChangeEvent] {
        self.change_history.as_ref()
    }
}

impl Contact {
    /// Create a new Contact.
    pub fn new(identifier: IdentityIdentifier, change_events: Vec<IdentityChangeEvent>) -> Self {
        Contact {
            identifier,
            change_history: IdentityChangeHistory::new(change_events),
        }
    }
}

impl Contact {
    /// Verify cryptographically whole event chain. Also verify sequence correctness
    pub async fn verify(&self, vault: &mut impl IdentityVault) -> Result<bool> {
        if !IdentityChangeHistory::check_consistency(&[], self.change_events()) {
            return deny();
        }

        if !self
            .change_history
            .verify_all_existing_events(vault)
            .await?
        {
            return deny();
        }

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = vault
            .compute_key_id_for_public_key(&root_public_key)
            .await?;
        let identity_id = IdentityIdentifier::from_key_id(root_key_id);

        if &identity_id != self.identifier() {
            return deny(); // IdentityIdDoesNotMatch Err(IdentityError::.into());
        }

        allow()
    }

    /// Update [`Contact`] by using new change events
    pub async fn verify_and_update<C: AsRef<[IdentityChangeEvent]>>(
        &mut self,
        change_events: C,
        vault: &mut impl IdentityVault,
    ) -> Result<bool> {
        if !IdentityChangeHistory::check_consistency(self.change_events(), change_events.as_ref()) {
            return deny();
        }

        for event in change_events.as_ref().iter() {
            if !IdentityChangeHistory::verify_event(self.change_events(), event, vault).await? {
                return deny();
            }
            self.change_history.push_event(event.clone());
        }

        allow()
    }
}

impl Contact {
    /// Get [`crate::Identity`] Update [`PublicKey`]
    pub fn get_identity_update_public_key(&self) -> Result<PublicKey> {
        IdentityChangeHistory::get_current_root_public_key(self.change_events())
    }
    /// Get [`PublicKey`]. Key is uniquely identified by the specified label.
    pub fn get_public_key(&self, label: &str) -> Result<PublicKey> {
        self.change_history.get_public_key(label)
    }
    /// Get [`EventIdentifier`] of the last known event
    pub fn get_last_event_id(&self) -> Result<EventIdentifier> {
        self.change_history.get_last_event_id()
    }
    /// Get BBS+ signing public key
    #[cfg(feature = "credentials")]
    pub fn get_signing_public_key(&self) -> Result<PublicKey> {
        use crate::Identity;
        self.get_public_key(Identity::CREDENTIALS_ISSUE)
    }
}
