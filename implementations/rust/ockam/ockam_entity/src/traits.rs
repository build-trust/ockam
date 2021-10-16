use crate::{Changes, Contact, Lease, ProfileChangeEvent, ProfileIdentifier, TrustPolicy, TTL};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Address, Result, Route};
use ockam_vault_core::{PublicKey, Secret};

pub type AuthenticationProof = Vec<u8>;

/// Identity
pub trait Identity: Send + 'static {
    /// Return unique [`Profile`](crate::Profile) identifier, which is equal to sha256 of the root public key
    fn identifier(&self) -> Result<ProfileIdentifier>;

    /// Create new key.
    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()>;

    /// Rotate existing key.
    fn rotate_profile_key(&mut self) -> Result<()>;

    /// Get [`Secret`] key.
    fn get_profile_secret_key(&self) -> Result<Secret>;

    /// Get [`Secret`] key.
    fn get_secret_key<S: Into<String>>(&self, label: S) -> Result<Secret>;

    /// Get [`PublicKey`].
    fn get_profile_public_key(&self) -> Result<PublicKey>;

    /// Get [`PublicKey`].
    fn get_public_key<S: Into<String>>(&self, label: S) -> Result<PublicKey>;

    /// Create an authentication proof based on the given state
    fn create_auth_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<AuthenticationProof>;

    /// Verify a proof based on the given state, proof and profile.
    fn verify_auth_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
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

    /// Convert [`Profile`](crate::Profile) to [`Contact`]
    fn as_contact(&mut self) -> Result<Contact>;

    /// Return [`Contact`] with given [`ProfileIdentifier`]
    fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>>;

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and add new [`Contact`] to [`Profile`](crate::Profile)'s Contact list
    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool>;

    /// Verify and update known [`Contact`] with new [`ProfileChangeEvent`]s
    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool>;

    fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: impl ToString,
        bucket: impl ToString,
        ttl: TTL,
    ) -> Result<Lease>;

    fn revoke_lease(&mut self, lease_manager_route: &Route, lease: Lease) -> Result<()>;
}

pub trait SecureChannels {
    fn create_secure_channel_listener(
        &mut self,
        address: impl Into<Address> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<()>;

    fn create_secure_channel(
        &mut self,
        route: impl Into<Route> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address>;
}
