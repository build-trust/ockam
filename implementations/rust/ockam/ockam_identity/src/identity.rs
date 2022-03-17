use crate::{
    AuthenticationProof, Changes, Contact, IdentityChangeEvent, IdentityChannelListener,
    IdentityIdentifier, IdentityState, IdentityTrait, IdentityVault, Lease, SecureChannelWorker,
    TrustPolicy, TTL,
};
use ockam_core::compat::{string::String, sync::Arc, vec::Vec};
use ockam_core::vault::{PublicKey, Secret};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, AsyncTryClone, Result, Route};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;

#[derive(AsyncTryClone)]
pub struct Identity<V: IdentityVault> {
    ctx: Context,
    state: Arc<RwLock<IdentityState<V>>>,
}

impl<V: IdentityVault> Identity<V> {
    pub async fn create(ctx: &Context, vault: &V) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;
        let state = IdentityState::create(vault.async_try_clone().await?).await?;
        Ok(Self {
            ctx: child_ctx,
            state: Arc::new(RwLock::new(state)),
        })
    }
}

#[async_trait]
impl<V: IdentityVault> IdentityTrait for Identity<V> {
    async fn identifier(&self) -> Result<IdentityIdentifier> {
        self.state.read().await.identifier().await
    }

    async fn create_key(&self, label: String) -> Result<()> {
        self.state.write().await.create_key(label).await
    }

    async fn add_key(&self, label: String, secret: &Secret) -> Result<()> {
        self.state.write().await.add_key(label, secret).await
    }

    async fn rotate_root_secret_key(&self) -> Result<()> {
        self.state.write().await.rotate_root_secret_key().await
    }

    async fn get_root_secret_key(&self) -> Result<Secret> {
        self.state.read().await.get_root_secret_key().await
    }

    async fn get_secret_key(&self, label: String) -> Result<Secret> {
        self.state.read().await.get_secret_key(label).await
    }

    async fn get_root_public_key(&self) -> Result<PublicKey> {
        self.state.read().await.get_root_public_key().await
    }

    async fn get_public_key(&self, label: String) -> Result<PublicKey> {
        self.state.read().await.get_public_key(label).await
    }

    async fn create_auth_proof(&self, state_slice: &[u8]) -> Result<AuthenticationProof> {
        self.state
            .write()
            .await
            .create_auth_proof(state_slice)
            .await
    }

    async fn verify_auth_proof(
        &self,
        state_slice: &[u8],
        peer_id: &IdentityIdentifier,
        proof_slice: &[u8],
    ) -> Result<bool> {
        self.state
            .write()
            .await
            .verify_auth_proof(state_slice, peer_id, proof_slice)
            .await
    }

    async fn add_change(&self, change_event: IdentityChangeEvent) -> Result<()> {
        self.state.write().await.add_change(change_event).await
    }

    async fn get_changes(&self) -> Result<Changes> {
        self.state.read().await.get_changes().await
    }

    async fn verify_changes(&self) -> Result<bool> {
        self.state.write().await.verify_changes().await
    }

    async fn get_contacts(&self) -> Result<Vec<Contact>> {
        self.state.read().await.get_contacts().await
    }

    async fn as_contact(&self) -> Result<Contact> {
        self.state.write().await.as_contact().await
    }

    async fn get_contact(&self, contact_id: &IdentityIdentifier) -> Result<Option<Contact>> {
        self.state.write().await.get_contact(contact_id).await
    }

    async fn verify_contact(&self, contact: Contact) -> Result<bool> {
        self.state.write().await.verify_contact(contact).await
    }

    async fn verify_and_add_contact(&self, contact: Contact) -> Result<bool> {
        self.state
            .write()
            .await
            .verify_and_add_contact(contact)
            .await
    }

    async fn verify_and_update_contact(
        &self,
        identity_id: &IdentityIdentifier,
        changes: &[IdentityChangeEvent],
    ) -> Result<bool> {
        self.state
            .write()
            .await
            .verify_and_update_contact(identity_id, changes)
            .await
    }

    async fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: String,
        bucket: String,
        ttl: TTL,
    ) -> Result<Lease> {
        self.state
            .read()
            .await
            .get_lease(lease_manager_route, org_id, bucket, ttl)
            .await
    }

    async fn revoke_lease(&self, lease_manager_route: &Route, lease: Lease) -> Result<()> {
        self.state
            .write()
            .await
            .revoke_lease(lease_manager_route, lease)
            .await
    }
}

impl<V: IdentityVault> Identity<V> {
    pub async fn create_secure_channel_listener(
        &self,
        address: impl Into<Address>,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let vault = self.state.read().await.vault.async_try_clone().await?;
        let identity_clone = self.async_try_clone().await?;
        let listener = IdentityChannelListener::new(trust_policy, identity_clone, vault);
        self.ctx.start_worker(address.into(), listener).await?;

        Ok(())
    }

    pub async fn create_secure_channel(
        &self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address> {
        let vault = self.state.read().await.vault.async_try_clone().await?;
        let identity_clone = self.async_try_clone().await?;

        SecureChannelWorker::create_initiator(
            &self.ctx,
            route.into(),
            identity_clone,
            trust_policy,
            vault,
        )
        .await
    }
}
