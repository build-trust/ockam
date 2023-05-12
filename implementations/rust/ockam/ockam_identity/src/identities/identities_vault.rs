use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Result};
use ockam_vault::{
    AsymmetricVault, EphemeralSecretsStore, Implementation, KeyId, PersistentSecretsStore,
    SecretsStore, SecretsStoreReader, SymmetricVault,
};
use ockam_vault::{PublicKey, Secret, SecretAttributes};
use ockam_vault::{Signature, Signer, StoredSecret};

/// Traits required for a Vault implementation suitable for use in an Identity
/// Vault with XX required functionality
pub trait IdentitiesVault: XXVault + PersistentSecretsStore + Signer {}

impl<D> IdentitiesVault for D where D: XXVault + PersistentSecretsStore + Signer {}

/// Vault with XX required functionality
pub trait XXVault: SecretsStore + AsymmetricVault + SymmetricVault + Send + Sync + 'static {}

impl<D> XXVault for D where
    D: SecretsStore + AsymmetricVault + SymmetricVault + Send + Sync + 'static
{
}

/// Vault with required functionalities after XX key exchange
pub trait XXInitializedVault: SecretsStore + SymmetricVault + Send + Sync + 'static {}

impl<D> XXInitializedVault for D where D: SecretsStore + SymmetricVault + Send + Sync + 'static {}

/// This struct is used to compensate for the lack of non-experimental trait upcasting in Rust
/// We encapsulate an IdentitiesVault and delegate the implementation of all the functions of
/// the various traits inherited by IdentitiesVault: SymmetricVault, SecretVault, etc...
struct CoercedIdentitiesVault {
    vault: Arc<dyn IdentitiesVault>,
}

impl Implementation for CoercedIdentitiesVault {}

#[async_trait]
impl EphemeralSecretsStore for CoercedIdentitiesVault {
    async fn create_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.create_ephemeral_secret(attributes).await
    }

    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        self.vault.import_ephemeral_secret(secret, attributes).await
    }

    async fn get_ephemeral_secret(
        &self,
        key_id: &KeyId,
        description: &str,
    ) -> Result<StoredSecret> {
        self.vault.get_ephemeral_secret(key_id, description).await
    }

    async fn delete_ephemeral_secret(&self, key_id: KeyId) -> Result<bool> {
        self.vault.delete_ephemeral_secret(key_id).await
    }
}

#[async_trait]
impl PersistentSecretsStore for CoercedIdentitiesVault {
    async fn create_persistent_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.create_persistent_secret(attributes).await
    }

    async fn delete_persistent_secret(&self, key_id: KeyId) -> Result<bool> {
        self.vault.delete_persistent_secret(key_id).await
    }
}

#[async_trait]
impl SecretsStoreReader for CoercedIdentitiesVault {
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.vault.get_secret_attributes(key_id).await
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.get_public_key(key_id).await
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.vault.get_key_id(public_key).await
    }
}

#[async_trait]
impl Signer for CoercedIdentitiesVault {
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        self.vault.sign(key_id, data).await
    }
    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        self.vault.verify(public_key, data, signature).await
    }
}

/// Return this vault as a symmetric vault
pub fn to_symmetric_vault(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn SymmetricVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}

/// Return this vault as a XX vault
pub fn to_xx_vault(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn XXVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}

/// Returns this vault as a XX initialized vault
pub fn to_xx_initialized(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn XXInitializedVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}
