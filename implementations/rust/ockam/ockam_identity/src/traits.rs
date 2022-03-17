use crate::{Changes, Contact, IdentityChangeEvent, IdentityIdentifier, Lease, TTL};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::vault::{PublicKey, Secret};
use ockam_core::{async_trait, compat::boxed::Box, AsyncTryClone};
use ockam_core::{Result, Route};

pub type AuthenticationProof = Vec<u8>;

/// Identity
#[async_trait]
pub trait IdentityTrait: AsyncTryClone + Send + Sync + 'static {
    /// Return unique [`Identity`](crate::Identity) identifier, which is equal to sha256 of the root public key
    async fn identifier(&self) -> Result<IdentityIdentifier>;

    /// Create new key.
    async fn create_key(&self, label: String) -> Result<()>;

    /// Add key that already exists in current Vault
    async fn add_key(&self, label: String, secret: &Secret) -> Result<()>;

    /// Rotate existing key.
    async fn rotate_root_secret_key(&self) -> Result<()>;

    /// Get [`Secret`] key.
    async fn get_root_secret_key(&self) -> Result<Secret>;

    /// Get [`Secret`] key.
    async fn get_secret_key(&self, label: String) -> Result<Secret>;

    /// Get [`PublicKey`].
    async fn get_root_public_key(&self) -> Result<PublicKey>;

    /// Get [`PublicKey`].
    async fn get_public_key(&self, label: String) -> Result<PublicKey>;

    /// Create an authentication proof based on the given state
    async fn create_auth_proof(&self, state_slice: &[u8]) -> Result<AuthenticationProof>;

    /// Verify a proof based on the given state, proof and identity.
    async fn verify_auth_proof(
        &self,
        state_slice: &[u8],
        peer_id: &IdentityIdentifier,
        proof_slice: &[u8],
    ) -> Result<bool>;

    /// Add a change event.
    async fn add_change(&self, change_event: IdentityChangeEvent) -> Result<()>;

    /// Return change history chain
    async fn get_changes(&self) -> Result<Changes>;

    /// Verify the whole change event chain
    async fn verify_changes(&self) -> Result<bool>;

    /// Return all known to this identity [`Contact`]s
    async fn get_contacts(&self) -> Result<Vec<Contact>>;

    /// Convert [`Identity`](crate::Identity) to [`Contact`]
    async fn as_contact(&self) -> Result<Contact>;

    /// Return [`Contact`] with given [`IdentityIdentifier`]
    async fn get_contact(&self, contact_id: &IdentityIdentifier) -> Result<Option<Contact>>;

    /// Verify cryptographically whole event chain. Also verify sequence correctness
    async fn verify_contact(&self, contact: Contact) -> Result<bool>;

    /// Verify and add new [`Contact`] to [`Identity`](crate::Identity)'s Contact list
    async fn verify_and_add_contact(&self, contact: Contact) -> Result<bool>;

    /// Verify and update known [`Contact`] with new [`IdentityChangeEvent`]s
    async fn verify_and_update_contact(
        &self,
        contact_id: &IdentityIdentifier,
        change_events: &[IdentityChangeEvent],
    ) -> Result<bool>;

    async fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: String,
        bucket: String,
        ttl: TTL,
    ) -> Result<Lease>;

    async fn revoke_lease(&self, lease_manager_route: &Route, lease: Lease) -> Result<()>;
}
