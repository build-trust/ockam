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
/// TODO
///
/// Authentication using [`Profile`]. In following example Bob authenticates Alice.
/// TODO
///
/// Update [`Profile`] and send changes to other parties. In following example Alice rotates
/// her key and sends corresponding [`Profile`] changes to Bob.
/// TODO
///
use crate::{
    AuthenticationProof, Changes, Contact, Entity, Identity, IdentityRequest, IdentityResponse,
    Lease, ProfileChangeEvent, ProfileIdentifier, TTL,
};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::{async_trait, compat::boxed::Box, AsyncTryClone};
use ockam_core::{Result, Route};
use ockam_node::Handle;
use ockam_vault::{PublicKey, Secret};

#[derive(AsyncTryClone)]
pub struct Profile {
    id: ProfileIdentifier,
    handle: Handle,
}

impl Entity {
    async fn from_profile(p: &Profile) -> Result<Entity> {
        Ok(Entity::new(
            p.handle.async_try_clone().await?,
            Some(p.id.clone()),
        ))
    }
}

impl Profile {
    pub fn new<I: Into<ProfileIdentifier>>(id: I, handle: Handle) -> Self {
        let id = id.into();
        Profile { id, handle }
    }

    pub async fn entity(&self) -> Result<Entity> {
        Entity::from_profile(self).await
    }

    pub async fn call(&self, req: IdentityRequest) -> Result<IdentityResponse> {
        self.handle.call(req).await
    }

    pub async fn cast(&self, req: IdentityRequest) -> Result<()> {
        self.handle.cast(req).await
    }
}

impl Profile {
    /// Sha256 of that value is used as previous event id for first event in a [`Profile`]
    pub const NO_EVENT: &'static [u8] = "OCKAM_NO_EVENT".as_bytes();
    /// Label for [`Profile`] update key
    pub const ROOT_LABEL: &'static str = "OCKAM_RK";
    /// Label for key used to issue credentials
    #[cfg(feature = "credentials")]
    pub const CREDENTIALS_ISSUE: &'static str = "OCKAM_CIK";
    /// Current version of change structure
    pub const CURRENT_CHANGE_VERSION: u8 = 1;
}

#[async_trait]
impl Identity for Profile {
    async fn identifier(&self) -> Result<ProfileIdentifier> {
        // FIXME: Clone on every call
        self.entity().await?.identifier().await
    }

    async fn create_key(&mut self, label: String) -> Result<()> {
        self.entity().await?.create_key(label).await
    }

    async fn add_key(&mut self, label: String, secret: &Secret) -> Result<()> {
        self.entity().await?.add_key(label, secret).await
    }

    async fn rotate_root_secret_key(&mut self) -> Result<()> {
        self.entity().await?.rotate_root_secret_key().await
    }

    async fn get_root_secret_key(&self) -> Result<Secret> {
        self.entity().await?.get_root_secret_key().await
    }

    async fn get_secret_key(&self, label: String) -> Result<Secret> {
        self.entity().await?.get_secret_key(label).await
    }

    async fn get_root_public_key(&self) -> Result<PublicKey> {
        self.entity().await?.get_root_public_key().await
    }

    async fn get_public_key(&self, label: String) -> Result<PublicKey> {
        self.entity().await?.get_public_key(label).await
    }

    async fn create_auth_proof(&mut self, state_slice: &[u8]) -> Result<AuthenticationProof> {
        self.entity().await?.create_auth_proof(state_slice).await
    }

    async fn verify_auth_proof(
        &mut self,
        state_slice: &[u8],
        peer_id: &ProfileIdentifier,
        proof_slice: &[u8],
    ) -> Result<bool> {
        self.entity()
            .await?
            .verify_auth_proof(state_slice, peer_id, proof_slice)
            .await
    }

    async fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        self.entity().await?.add_change(change_event).await
    }

    async fn get_changes(&self) -> Result<Changes> {
        self.entity().await?.get_changes().await
    }

    async fn verify_changes(&mut self) -> Result<bool> {
        self.entity().await?.verify_changes().await
    }

    async fn get_contacts(&self) -> Result<Vec<Contact>> {
        self.entity().await?.get_contacts().await
    }

    async fn as_contact(&mut self) -> Result<Contact> {
        let changes = self.get_changes().await?;
        Ok(Contact::new(self.id.clone(), changes))
    }

    async fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>> {
        self.entity().await?.get_contact(contact_id).await
    }

    async fn verify_contact(&mut self, contact: Contact) -> Result<bool> {
        self.entity().await?.verify_contact(contact).await
    }

    async fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        self.entity().await?.verify_and_add_contact(contact).await
    }

    async fn verify_and_update_contact(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: &[ProfileChangeEvent],
    ) -> Result<bool> {
        self.entity()
            .await?
            .verify_and_update_contact(contact_id, change_events)
            .await
    }

    async fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: String,
        bucket: String,
        ttl: TTL,
    ) -> Result<Lease> {
        self.entity()
            .await?
            .get_lease(lease_manager_route, org_id, bucket, ttl)
            .await
    }

    async fn revoke_lease(&mut self, lease_manager_route: &Route, lease: Lease) -> Result<()> {
        self.entity()
            .await?
            .revoke_lease(lease_manager_route, lease)
            .await
    }
}
