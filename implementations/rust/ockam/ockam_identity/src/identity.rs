use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::IdentityChangeEvent;
use crate::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use crate::{
    EventIdentifier, IdentityError, IdentityEventAttributes, IdentityIdentifier, IdentityVault,
    KeyAttributes, MetaKeyAttributes,
};
use ockam_core::compat::{
    boxed::Box,
    rand::{thread_rng, CryptoRng, RngCore},
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use ockam_core::vault::{SecretPersistence, SecretType, Signature, CURVE25519_SECRET_LENGTH};
use ockam_core::AsyncTryClone;
use ockam_core::{Address, Result};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_vault::{KeyId, PublicKey, SecretAttributes};

/// Identity implementation
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct Identity<V: IdentityVault> {
    id: IdentityIdentifier,
    pub(crate) change_history: Arc<RwLock<IdentityChangeHistory>>,
    pub(crate) ctx: Context,
    pub(crate) vault: V,
}

pub struct IdentityStateConst;

impl IdentityStateConst {
    /// Sha256 of that value is used as previous event id for first event in a
    /// [`crate::Identity`]
    pub const NO_EVENT: &'static [u8] = "OCKAM_NO_EVENT".as_bytes();
    /// Label for [`crate::Identity`] update key
    pub const ROOT_LABEL: &'static str = "OCKAM_RK";
    /// Current version of change structure
    pub const CURRENT_CHANGE_VERSION: u8 = 1;
    /// Change history key for AuthenticatedStorage
    pub const CHANGE_HISTORY_KEY: &'static str = "CHANGE_HISTORY";
}

impl<V: IdentityVault> Identity<V> {
    /// Identity constructor
    pub fn new(
        id: IdentityIdentifier,
        change_history: IdentityChangeHistory,
        ctx: Context,
        vault: V,
        rng: impl RngCore + CryptoRng + Clone,
    ) -> Self {
        // Avoid warning
        let _ = rng;

        Self {
            id,
            change_history: Arc::new(RwLock::new(change_history)),
            ctx,
            vault,
        }
    }

    pub async fn export(&self) -> Result<Vec<u8>> {
        self.change_history.read().await.export()
    }

    pub async fn import(ctx: &Context, data: &[u8], vault: &V) -> Result<Self> {
        let change_history = IdentityChangeHistory::import(data)?;
        if !change_history.verify_all_existing_events(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }
        let child_ctx = ctx.new_detached(Address::random_local()).await?;

        let id = change_history.compute_identity_id(vault).await?;

        let vault = vault.async_try_clone().await?;

        let identity = Self::new(id, change_history, child_ctx, vault, thread_rng());

        Ok(identity)
    }

    pub async fn changes(&self) -> Result<IdentityChangeHistory> {
        Ok(self.change_history.read().await.clone())
    }

    pub fn vault(&self) -> &V {
        &self.vault
    }

    pub async fn verify_changes(&self) -> Result<bool> {
        self.change_history
            .read()
            .await
            .verify_all_existing_events(&self.vault)
            .await
    }

    /// Create Identity
    pub async fn create(ctx: &Context, vault: &V) -> Result<Self> {
        let child_ctx = ctx.new_detached(Address::random_local()).await?;
        let initial_event_id = EventIdentifier::initial(vault).await;

        let key_attribs = KeyAttributes::new(
            IdentityStateConst::ROOT_LABEL.to_string(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        );

        let create_key_event = Self::make_create_key_event_static(
            None,
            initial_event_id,
            key_attribs.clone(),
            IdentityEventAttributes::new(),
            None,
            vault,
        )
        .await?;

        let change_history = IdentityChangeHistory::new(create_key_event);

        // Sanity check
        if !change_history.check_entire_consistency() {
            return Err(IdentityError::ConsistencyError.into());
        }

        // Sanity check
        if !change_history.verify_all_existing_events(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        let id = change_history.compute_identity_id(vault).await?;

        let vault = vault.async_try_clone().await?;

        let identity = Self::new(id, change_history, child_ctx, vault, thread_rng());

        Ok(identity)
    }
}

impl<V: IdentityVault> Identity<V> {
    pub(crate) async fn get_secret_key_from_event(
        event: &IdentityChangeEvent,
        vault: &V,
    ) -> Result<KeyId> {
        let public_key = event.change_block().change().public_key()?;

        vault.compute_key_id_for_public_key(&public_key).await
    }

    async fn add_change(&self, change_event: IdentityChangeEvent) -> Result<()> {
        self.change_history
            .write()
            .await
            .check_consistency_and_add_event(change_event)
    }
}

impl<V: IdentityVault> Identity<V> {
    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.id
    }

    pub async fn create_key(&self, label: String) -> Result<()> {
        let key_attribs = KeyAttributes::default_with_label(label);

        let event = self
            .make_create_key_event(None, key_attribs, IdentityEventAttributes::new())
            .await?;

        self.add_change(event).await
    }

    pub async fn add_key(&self, label: String, secret: &KeyId) -> Result<()> {
        let secret_attributes = self.vault.secret_attributes_get(secret).await?;
        let key_attribs = KeyAttributes::new(
            label,
            MetaKeyAttributes::SecretAttributes(secret_attributes),
        );

        let event = self
            .make_create_key_event(Some(secret), key_attribs, IdentityEventAttributes::new())
            .await?;

        self.add_change(event).await
    }

    pub async fn rotate_root_secret_key(&self) -> Result<()> {
        let event = self
            .make_rotate_key_event(
                KeyAttributes::default_with_label(IdentityStateConst::ROOT_LABEL.to_string()),
                IdentityEventAttributes::new(),
            )
            .await?;

        self.add_change(event).await
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    pub async fn get_root_secret_key(&self) -> Result<KeyId> {
        self.get_secret_key(IdentityStateConst::ROOT_LABEL.to_string())
            .await
    }

    pub async fn get_secret_key(&self, label: String) -> Result<KeyId> {
        let event = IdentityChangeHistory::find_last_key_event(
            self.change_history.read().await.as_ref(),
            &label,
        )?
        .clone();
        Self::get_secret_key_from_event(&event, &self.vault).await
    }

    pub async fn get_root_public_key(&self) -> Result<PublicKey> {
        self.change_history.read().await.get_root_public_key()
    }

    pub async fn get_public_key(&self, label: String) -> Result<PublicKey> {
        self.change_history.read().await.get_public_key(&label)
    }

    /// Generate Proof of possession of [`crate::Identity`].
    ///
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    pub async fn create_signature(&self, data: &[u8]) -> Result<Signature> {
        let root_secret = self.get_root_secret_key().await?;

        self.vault.sign(&root_secret, data).await
    }

    /// Verify Proof of possession of [`Identity`](crate::Identity) with given
    /// [`IdentityIdentifier`]. channel_state should be tied to channel's
    /// cryptographical material (e.g. h value for Noise XX)
    pub async fn verify_signature(
        &self,
        signature: &Signature,
        their_identity_id: &IdentityIdentifier,
        data: &[u8],
        storage: &impl AuthenticatedStorage,
    ) -> Result<bool> {
        let their_identity = match self.get_known_identity(their_identity_id, storage).await? {
            Some(i) => i,
            None => return Err(IdentityError::IdentityNotFound.into()),
        };
        let public_key = their_identity.get_root_public_key()?;

        self.vault.verify(signature, &public_key, data).await
    }

    pub async fn get_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
        storage: &impl AuthenticatedStorage,
    ) -> Result<Option<IdentityChangeHistory>> {
        if let Some(known) = storage
            .get(
                &their_identity_id.to_string(),
                IdentityStateConst::CHANGE_HISTORY_KEY,
            )
            .await?
        {
            let known = IdentityChangeHistory::import(&known)?;

            if !known.verify_all_existing_events(&self.vault).await? {
                return Err(IdentityError::IdentityVerificationFailed.into());
            }

            Ok(Some(known))
        } else {
            Ok(None)
        }
    }

    pub async fn update_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
        current_history: &IdentityChangeHistory,
        storage: &impl AuthenticatedStorage,
    ) -> Result<()> {
        let should_set =
            if let Some(known) = self.get_known_identity(their_identity_id, storage).await? {
                match current_history.compare(&known) {
                    IdentityHistoryComparison::Equal => false, /* Do nothing */
                    IdentityHistoryComparison::Conflict => {
                        return Err(IdentityError::ConsistencyError.into())
                    }
                    IdentityHistoryComparison::Newer => true, /* Update */
                    IdentityHistoryComparison::Older => {
                        return Err(IdentityError::ConsistencyError.into())
                    }
                }
            } else {
                true
            };

        if should_set {
            storage
                .set(
                    &their_identity_id.to_string(),
                    IdentityStateConst::CHANGE_HISTORY_KEY.to_string(),
                    current_history.export()?,
                )
                .await?;
        }

        Ok(())
    }
}
