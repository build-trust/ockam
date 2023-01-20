use crate::authenticated_storage::mem::InMemoryStorage;
use crate::authenticated_storage::AuthenticatedStorage;
use crate::change::IdentitySignedChange;
use crate::change_history::{IdentityChangeHistory, IdentityHistoryComparison};
use crate::credential::Credential;
use crate::{
    ChangeIdentifier, IdentityError, IdentityIdentifier, IdentityVault, KeyAttributes,
    PublicIdentity, SecureChannelRegistry,
};
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use ockam_core::vault::{SecretPersistence, SecretType, Signature, CURVE25519_SECRET_LENGTH_U32};
use ockam_core::{Address, Result};
use ockam_core::{AsyncTryClone, DenyAll};
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::Context;
use ockam_vault::{KeyId, SecretAttributes};

/// Identity implementation
#[derive(AsyncTryClone)]
#[async_try_clone(crate = "ockam_core")]
pub struct Identity<V: IdentityVault, S: AuthenticatedStorage> {
    id: IdentityIdentifier,
    pub(crate) credential: Arc<RwLock<Option<Credential<'static>>>>,
    pub(crate) change_history: Arc<RwLock<IdentityChangeHistory>>,
    pub(crate) ctx: Context,
    pub(crate) authenticated_storage: S,
    pub(crate) secure_channel_registry: SecureChannelRegistry,
    pub(crate) vault: V,
}

/// `Identity`-related constants
pub struct IdentityStateConst;

impl IdentityStateConst {
    /// Sha256 of that value is used as previous change id for first change in a
    /// [`crate::Identity`]
    pub const INITIAL_CHANGE: &'static [u8] = "OCKAM_INITIAL_CHANGE".as_bytes();
    /// Label for [`crate::Identity`] update key
    pub const ROOT_LABEL: &'static str = "OCKAM_RK";
    /// Current version of change structure
    pub const CURRENT_CHANGE_VERSION: u8 = 1;
    /// Change history key for AuthenticatedStorage
    pub const CHANGE_HISTORY_KEY: &'static str = "CHANGE_HISTORY";
    /// Attributes key for AuthenticatedStorage
    pub const ATTRIBUTES_KEY: &'static str = "ATTRIBUTES";
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// Identity constructor
    pub(crate) fn new(
        id: IdentityIdentifier,
        change_history: IdentityChangeHistory,
        ctx: Context,
        authenticated_storage: S,
        secure_channel_registry: SecureChannelRegistry,
        vault: V,
    ) -> Self {
        Self {
            id,
            credential: Arc::new(RwLock::new(None)),
            change_history: Arc::new(RwLock::new(change_history)),
            ctx,
            authenticated_storage,
            secure_channel_registry,
            vault,
        }
    }

    /// Import and verify an `Identity` from the binary format. Extended version
    pub async fn import_ext(
        ctx: &Context,
        data: &[u8],
        authenticated_storage: &S,
        secure_channel_registry: &SecureChannelRegistry,
        vault: &V,
    ) -> Result<Self> {
        let change_history = IdentityChangeHistory::import(data)?;
        if !change_history.verify_all_existing_changes(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Identity.import.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;

        let id = change_history.compute_identity_id(vault).await?;

        let vault = vault.async_try_clone().await?;

        let identity = Self::new(
            id,
            change_history,
            child_ctx,
            authenticated_storage.async_try_clone().await?,
            secure_channel_registry.clone(),
            vault,
        );

        Ok(identity)
    }

    async fn create_impl(
        ctx: &Context,
        authenticated_storage: S,
        secure_channel_registry: SecureChannelRegistry,
        vault: &V,
        kid: Option<&KeyId>,
        key_attribs: KeyAttributes,
    ) -> Result<Self> {
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("Identity.create.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;
        let initial_change_id = ChangeIdentifier::initial(vault).await;

        let create_key_change = Self::make_create_key_change_static(
            kid,
            initial_change_id,
            key_attribs.clone(),
            None,
            vault,
        )
        .await?;

        let change_history = IdentityChangeHistory::new(create_key_change);

        // Sanity check
        if !change_history.check_entire_consistency() {
            return Err(IdentityError::ConsistencyError.into());
        }

        // Sanity check
        if !change_history.verify_all_existing_changes(vault).await? {
            return Err(IdentityError::IdentityVerificationFailed.into());
        }

        let id = change_history.compute_identity_id(vault).await?;

        let vault = vault.async_try_clone().await?;

        let identity = Self::new(
            id,
            change_history,
            child_ctx,
            authenticated_storage,
            secure_channel_registry,
            vault,
        );

        Ok(identity)
    }

    /// Create an `Identity`. Extended version
    pub async fn create_ext(ctx: &Context, authenticated_storage: &S, vault: &V) -> Result<Self> {
        let attrs = KeyAttributes::new(
            IdentityStateConst::ROOT_LABEL.to_string(),
            SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH_U32,
            ),
        );
        Self::create_impl(
            ctx,
            authenticated_storage.async_try_clone().await?,
            SecureChannelRegistry::new(),
            vault,
            None,
            attrs,
        )
        .await
    }

    /// Create an `Identity` with an external key. Extended version
    pub async fn create_with_external_key_ext(
        ctx: &Context,
        authenticated_storage: &S,
        vault: &V,
        kid: &KeyId,
        attrs: KeyAttributes,
    ) -> Result<Self> {
        Self::create_impl(
            ctx,
            authenticated_storage.async_try_clone().await?,
            SecureChannelRegistry::new(),
            vault,
            Some(kid),
            attrs,
        )
        .await
    }
}

impl<V: IdentityVault> Identity<V, InMemoryStorage> {
    /// Create an `Identity` with a new secret key and `InMemoryStorage`
    pub async fn create(ctx: &Context, vault: &V) -> Result<Self> {
        let attrs = KeyAttributes::new(
            IdentityStateConst::ROOT_LABEL.to_string(),
            SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH_U32,
            ),
        );
        Self::create_impl(
            ctx,
            InMemoryStorage::new(),
            SecureChannelRegistry::new(),
            vault,
            None,
            attrs,
        )
        .await
    }

    /// Create an `Identity` with an external key.
    pub async fn create_with_external_key(
        ctx: &Context,
        vault: &V,
        kid: &KeyId,
        attrs: KeyAttributes,
    ) -> Result<Self> {
        Self::create_impl(
            ctx,
            InMemoryStorage::new(),
            SecureChannelRegistry::new(),
            vault,
            Some(kid),
            attrs,
        )
        .await
    }

    /// Import and verify `Identity` from the binary format. Uses `InMemoryStorage`
    pub async fn import(ctx: &Context, data: &[u8], vault: &V) -> Result<Self> {
        Self::import_ext(
            ctx,
            data,
            &InMemoryStorage::new(),
            &SecureChannelRegistry::new(),
            vault,
        )
        .await
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// Export an `Identity` to the binary format
    pub async fn export(&self) -> Result<Vec<u8>> {
        self.change_history.read().await.export()
    }

    /// Vault
    pub fn vault(&self) -> &V {
        &self.vault
    }

    /// `AuthenticatedStorage`
    pub fn authenticated_storage(&self) -> &S {
        &self.authenticated_storage
    }

    /// `SecureChannelRegistry` with all known SecureChannels this `Identity` has created
    pub fn secure_channel_registry(&self) -> &SecureChannelRegistry {
        &self.secure_channel_registry
    }

    /// `Identity` change history
    pub async fn change_history(&self) -> IdentityChangeHistory {
        self.change_history.read().await.clone()
    }

    /// `Context`
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    pub(crate) async fn get_secret_key_from_change(
        change: &IdentitySignedChange,
        vault: &V,
    ) -> Result<KeyId> {
        let public_key = change.change().public_key()?;

        vault.compute_key_id_for_public_key(&public_key).await
    }

    async fn add_change(&self, change: IdentitySignedChange) -> Result<()> {
        self.change_history
            .write()
            .await
            .check_consistency_and_add_change(change)
    }
}

impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
    /// `IdentityIdentifier` of this `Identity`
    pub fn identifier(&self) -> &IdentityIdentifier {
        &self.id
    }

    /// Generate and add a new key to this `Identity` with a given `label`
    pub async fn create_key(&self, label: String) -> Result<()> {
        let key_attribs = KeyAttributes::default_with_label(label);

        let change = self.make_create_key_change(None, key_attribs).await?;

        self.add_change(change).await
    }

    /// Add a new key to this `Identity` with a given `label`
    pub async fn add_key(&self, label: String, secret: &KeyId) -> Result<()> {
        let secret_attributes = self.vault.secret_attributes_get(secret).await?;
        let key_attribs = KeyAttributes::new(label, secret_attributes);

        let change = self
            .make_create_key_change(Some(secret), key_attribs)
            .await?;

        self.add_change(change).await
    }

    /// Rotate an existing key with a given label
    pub async fn rotate_key(&self, label: &str) -> Result<()> {
        let change = self
            .make_rotate_key_change(KeyAttributes::default_with_label(label.to_string()))
            .await?;

        self.add_change(change).await
    }

    /// Rotate this `Identity` root key
    pub async fn rotate_root_key(&self) -> Result<()> {
        let change = self
            .make_rotate_key_change(KeyAttributes::default_with_label(
                IdentityStateConst::ROOT_LABEL.to_string(),
            ))
            .await?;

        self.add_change(change).await
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    pub(crate) async fn get_root_secret_key(&self) -> Result<KeyId> {
        self.get_secret_key(IdentityStateConst::ROOT_LABEL).await
    }

    pub(crate) async fn get_secret_key(&self, label: &str) -> Result<KeyId> {
        let change = IdentityChangeHistory::find_last_key_change(
            self.change_history.read().await.as_ref(),
            label,
        )?
        .clone();
        Self::get_secret_key_from_change(&change, &self.vault).await
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

    /// Get a `PublicIdentity` with given `IdentityIdentifier`,
    /// that we already should know (e.g. after creating a SecureChannel with it)
    pub async fn get_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
    ) -> Result<Option<PublicIdentity>> {
        if let Some(known) = self
            .authenticated_storage
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

    /// Update previously known `Identity` given newly obtained changes in `PublicIdentity`
    pub async fn update_known_identity(
        &self,
        their_identity_id: &IdentityIdentifier,
        current_history: &PublicIdentity,
    ) -> Result<()> {
        let should_set = if let Some(known) = self.get_known_identity(their_identity_id).await? {
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
            self.authenticated_storage
                .set(
                    &their_identity_id.to_string(),
                    IdentityStateConst::CHANGE_HISTORY_KEY.to_string(),
                    current_history.export()?,
                )
                .await?;
        }

        Ok(())
    }

    /// Export to `PublicIdentity`
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

    impl<V: IdentityVault, S: AuthenticatedStorage> Identity<V, S> {
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
                .verify_all_existing_changes(&self.vault)
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

        identity.rotate_root_key().await?;

        if !identity.verify_changes().await? {
            return test_error("verify_changes failed");
        }

        let secret3 = identity.get_root_secret_key().await?;
        let public3 = identity.get_root_public_key().await?;

        identity.rotate_root_key().await?;

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
