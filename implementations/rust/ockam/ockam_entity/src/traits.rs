use crate::{Changes, Contact, ProfileChangeEvent, ProfileIdentifier};
use ockam_core::{Address, Result, Route};
use ockam_vault_core::{PublicKey, Secret};

pub type Proof = Vec<u8>;

/// Identity
pub trait Identity: Send + 'static {
    /// Return unique [`Profile`] identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> Result<ProfileIdentifier>;

    /// Create new key.
    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()>;

    /// Rotate existing key.
    fn rotate_key(&mut self) -> Result<()>;

    /// Get [`Secret`] key.
    fn get_secret_key(&self) -> Result<Secret>;

    /// Get [`PublicKey`].
    fn get_public_key(&self) -> Result<PublicKey>;

    /// Create an authentication proof based on the given state
    fn create_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<Proof>;

    /// Verify a proof based on the given state, proof and profile.
    fn verify_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        state_slice: S,
        peer_id: &ProfileIdentifier,
        proof_slice: P,
    ) -> Result<bool>;

    /// Add a change event.
    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()>;

    /// Return change history chain
    fn get_changes(&self) -> Result<Changes>;

    /// Verify the whole change event chain
    fn verify_changes(&mut self) -> Result<bool>;

    /// Return all known to this profile [`Contact`]s
    fn get_contacts(&self) -> Result<Vec<Contact>>;

    /// Convert [`Profile`] to [`Contact`]
    fn as_contact(&mut self) -> Result<Contact>;

    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>>;

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and add new [`Contact`] to [`Profile`]'s Contact list
    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool>;
}

pub trait SecureChannels {
    fn create_secure_channel_listener<A: Into<Address> + Send>(&mut self, address: A)
        -> Result<()>;

    fn create_secure_channel<R: Into<Route> + Send>(&mut self, route: R) -> Result<Address>;
}
