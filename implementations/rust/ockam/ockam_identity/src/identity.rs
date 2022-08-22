use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::IdentityChangeEvent;
use crate::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use crate::credential::Credential;
use crate::{
    EventIdentifier, IdentityError, IdentityEventAttributes, IdentityIdentifier, IdentityVault,
    KeyAttributes, MetaKeyAttributes, PublicIdentity,
};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use ockam_core::vault::{SecretPersistence, SecretType, Signature, CURVE25519_SECRET_LENGTH};
use ockam_core::AsyncTryClone;
use ockam_core::{Address, Result};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_vault::{KeyId, SecretAttributes};

/// Identity implementation
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct Identity<V: IdentityVault> {
    id: IdentityIdentifier,
    pub(crate) credential: Arc<RwLock<Option<Credential<'static>>>>,
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
    /// Attributes key for AuthenticatedStorage
    pub const ATTRIBUTES_KEY: &'static str = "ATTRIBUTES";
}

impl<V: IdentityVault> Identity<V> {
    /// Identity constructor
    pub(crate) fn new(
        id: IdentityIdentifier,
        change_history: IdentityChangeHistory,
        ctx: Context,
        vault: V,
    ) -> Self {
        Self {
            id,
            credential: Arc::new(RwLock::new(None)),
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

        let identity = Self::new(id, change_history, child_ctx, vault);

        Ok(identity)
    }

    pub fn vault(&self) -> &V {
        &self.vault
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

        let identity = Self::new(id, change_history, child_ctx, vault);

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
    pub(crate) async fn get_root_secret_key(&self) -> Result<KeyId> {
        self.get_secret_key(IdentityStateConst::ROOT_LABEL).await
    }

    pub(crate) async fn get_secret_key(&self, label: &str) -> Result<KeyId> {
        let event = IdentityChangeHistory::find_last_key_event(
            self.change_history.read().await.as_ref(),
            label,
        )?
        .clone();
        Self::get_secret_key_from_event(&event, &self.vault).await
    }

    /// Generate Proof of possession of [`crate::Identity`].
    ///
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    pub async fn create_signature(
        &self,
        data: &[u8],
        key_label: Option<&str>,
    ) -> Result<Signature> {
        let secret = match key_label {
            Some(label) => self.get_secret_key(label).await?,
            None => self.get_root_secret_key().await?,
        };

        self.vault.sign(&secret, data).await
    }

    pub async fn get_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
        storage: &impl AuthenticatedStorage,
    ) -> Result<Option<PublicIdentity>> {
        if let Some(known) = storage
            .get(
                &their_identity_id.to_string(),
                IdentityStateConst::CHANGE_HISTORY_KEY,
            )
            .await?
        {
            let known = PublicIdentity::import(&known, &self.vault).await?;

            Ok(Some(known))
        } else {
            Ok(None)
        }
    }

    pub async fn update_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
        current_history: &PublicIdentity,
        storage: &impl AuthenticatedStorage,
    ) -> Result<()> {
        let should_set =
            if let Some(known) = self.get_known_identity(their_identity_id, storage).await? {
                match current_history.changes().compare(known.changes()) {
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

    pub async fn to_public(&self) -> Result<PublicIdentity> {
        Ok(PublicIdentity::new(
            self.id.clone(),
            self.change_history.read().await.clone(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ockam_core::errcode::{Kind, Origin};
    use ockam_core::vault::PublicKey;
    use ockam_core::Error;
    use ockam_vault::Vault;

    fn test_error<S: Into<String>>(error: S) -> Result<()> {
        Err(Error::new_without_cause(Origin::Identity, Kind::Unknown).context("msg", error.into()))
    }

    impl<V: IdentityVault> Identity<V> {
        pub async fn get_root_public_key(&self) -> Result<PublicKey> {
            self.change_history.read().await.get_root_public_key()
        }

        pub async fn get_public_key(&self, label: &str) -> Result<PublicKey> {
            self.change_history.read().await.get_public_key(label)
        }

        async fn verify_changes(&self) -> Result<bool> {
            self.change_history
                .read()
                .await
                .verify_all_existing_events(&self.vault)
                .await
        }
    }

    #[ockam_macros::test]
    async fn test_basic_identity_key_ops(ctx: &mut Context) -> Result<()> {
        let vault = Vault::create();

        let identity = Identity::create(ctx, &vault).await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret1 = identity.get_root_secret_key().await?;
        let public1 = identity.get_root_public_key().await?;

        identity.create_key("Truck management".to_string()).await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret2 = identity.get_secret_key("Truck management").await?;
        let public2 = identity.get_public_key("Truck management").await?;

        if secret1 == secret2 {
            return test_error("secret did not change after create_key");
        }

        if public1 == public2 {
            return test_error("public did not change after create_key");
        }

        identity.rotate_root_secret_key().await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret3 = identity.get_root_secret_key().await?;
        let public3 = identity.get_root_public_key().await?;

        identity.rotate_root_secret_key().await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        if secret1 == secret3 {
            return test_error("secret did not change after rotate_key");
        }

        if public1 == public3 {
            return test_error("public did not change after rotate_key");
        }

        ctx.stop().await?;

        Ok(())
    }
}
