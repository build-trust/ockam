use crate::{
    AsymmetricVault, Buffer, EphemeralSecretsStore, KeyId, PersistentSecretsStore, PublicKey,
    Secret, SecretAttributes, SecretsStore, SecretsStoreReader, SecurityModule, Signature, Signer,
    StoredSecret, SymmetricVault, VaultBuilder, VaultSecurityModule,
};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::fmt::Vec;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Result};
use ockam_node::KeyValueStorage;
#[cfg(feature = "std")]
use std::path::Path;

/// A Vault provides high-level interfaces to manage secrets:
///
///  - storage
///  - symmetric/asymmetric encryption
///  - signing
///
/// Its implementation is modular: storage can be replaced, signing can be provided via an
/// external KMS system etc...
///
/// # Examples
/// ```
/// use ockam_vault::{PersistentSecretsStore, SecretAttributes, SecretsStoreReader, Signer, Vault};
/// use ockam_core::Result;
///
/// async fn example() -> Result<()> {
///     let mut vault: Vault = Vault::default();
///
///     let mut attributes = SecretAttributes::X25519;
///
///     let secret = vault.create_persistent_secret(attributes).await?;
///     let public = vault.get_public_key(&secret).await?;
///
///     let data = "Very important stuff".as_bytes();
///
///     let signature = vault.sign(&secret, data).await?;
///     assert!(vault.verify(&public, data, &signature).await?);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Vault {
    /// implementation of a secret store
    pub(crate) secrets_store: Arc<dyn SecretsStore>,
    /// implementation of asymmetric encryption functionalities
    pub(crate) asymmetric_vault: Arc<dyn AsymmetricVault>,
    /// implementation of symmetric encryption functionalities
    pub(crate) symmetric_vault: Arc<dyn SymmetricVault>,
    /// implementation of signing encryption functionalities
    pub(crate) signer: Arc<dyn Signer>,
}

impl Default for Vault {
    fn default() -> Self {
        Vault::builder().make()
    }
}

/// Storage for Vault persistent values
pub type VaultStorage = Arc<dyn KeyValueStorage<KeyId, StoredSecret>>;

impl Vault {
    /// Create a new VaultBuilder to build a Vault from different implementations
    pub fn builder() -> VaultBuilder {
        VaultBuilder::new_builder()
    }

    /// Create a new default vault implementation
    pub fn new() -> Self {
        Vault::builder().make()
    }

    /// Create a new vault with an in memory storage, return as an Arc
    /// This is used in examples only where we don't need to really persist secrets
    pub fn create() -> Arc<Vault> {
        Vault::builder().build()
    }

    /// Create a new vault with a persistent storage
    #[cfg(feature = "std")]
    pub async fn create_with_persistent_storage_path(path: &Path) -> Result<Arc<Vault>> {
        Ok(Vault::builder()
            .with_persistent_storage_path(path)
            .await?
            .build())
    }

    /// Create a new vault with a specific storage
    pub fn create_with_persistent_storage(storage: VaultStorage) -> Arc<Vault> {
        Vault::builder().with_persistent_storage(storage).build()
    }

    /// Create a new vault with a specific security module backend
    pub fn create_with_security_module(security_module: Arc<dyn SecurityModule>) -> Arc<Vault> {
        Vault::builder()
            .with_security_module(security_module)
            .build()
    }

    /// The sha256 is a constant function which must always refer to the same implementation
    /// wherever it is used
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        VaultSecurityModule::sha256(data)
    }

    /// This function is compute_sha256 used in the ockam_vault_ffi crate
    /// where we always call functions on a Vault instance
    pub fn compute_sha256(&self, data: &[u8]) -> [u8; 32] {
        VaultSecurityModule::sha256(data)
    }
}

#[async_trait]
impl EphemeralSecretsStore for Vault {
    async fn create_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.secrets_store.create_ephemeral_secret(attributes).await
    }

    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        self.secrets_store
            .import_ephemeral_secret(secret, attributes)
            .await
    }

    async fn get_ephemeral_secret(
        &self,
        key_id: &KeyId,
        description: &str,
    ) -> Result<StoredSecret> {
        self.secrets_store
            .get_ephemeral_secret(key_id, description)
            .await
    }

    async fn delete_ephemeral_secret(&self, key_id: KeyId) -> Result<bool> {
        self.secrets_store.delete_ephemeral_secret(key_id).await
    }
}

#[async_trait]
impl PersistentSecretsStore for Vault {
    async fn create_persistent_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.secrets_store
            .create_persistent_secret(attributes)
            .await
    }

    async fn delete_persistent_secret(&self, key_id: KeyId) -> Result<bool> {
        self.secrets_store.delete_persistent_secret(key_id).await
    }
}

#[async_trait]
impl SecretsStoreReader for Vault {
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.secrets_store.get_secret_attributes(key_id).await
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.secrets_store.get_public_key(key_id).await
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.secrets_store.get_key_id(public_key).await
    }
}

#[async_trait]
impl AsymmetricVault for Vault {
    async fn ec_diffie_hellman(
        &self,
        secret: &KeyId,
        peer_public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.asymmetric_vault
            .ec_diffie_hellman(secret, peer_public_key)
            .await
    }

    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<KeyId>> {
        self.asymmetric_vault
            .hkdf_sha256(salt, info, ikm, output_attributes)
            .await
    }
}

#[async_trait]
impl SymmetricVault for Vault {
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.symmetric_vault
            .aead_aes_gcm_encrypt(key_id, plaintext, nonce, aad)
            .await
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.symmetric_vault
            .aead_aes_gcm_decrypt(key_id, cipher_text, nonce, aad)
            .await
    }
}

#[async_trait]
impl Signer for Vault {
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        self.signer.sign(key_id, data).await
    }

    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        self.signer.verify(public_key, data, signature).await
    }
}

/// This marker traits is used by implementations of Asymmetric and Symmetric traits.
/// Having this trait avoids conflicting instances of Vault which has an `AsymmetricVault` instance
/// delegating to its `asymmetric` member.
/// There is also a default `AsymmetricVault` instance for any `SecretsStore + Implementation`.
/// If it was just `SecretsStore` then `Vault` would also qualify for that instance because it has
/// a `SecretsStore` instance via its `secrets_store` member
pub trait Implementation {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::create_temp_file;
    use crate::storage::PersistentStorage;
    use crate::SecretAttributes;
    use ockam_core::compat::join;

    #[tokio::test]
    async fn test_vault_restart() {
        let storage = PersistentStorage::create(create_temp_file().as_path())
            .await
            .unwrap();
        let vault = Vault::create_with_persistent_storage(storage.clone());

        // create 3 secrets, 2 persistent, one ephemeral
        let attributes1 = SecretAttributes::Ed25519;
        let attributes2 = SecretAttributes::Ed25519;
        let attributes3 = SecretAttributes::Ed25519;

        let (key_id1, key_id2, key_id3) = join!(
            vault.create_persistent_secret(attributes1),
            vault.create_persistent_secret(attributes2),
            vault.create_ephemeral_secret(attributes3)
        );

        let key_id1 = key_id1.unwrap();
        let key_id2 = key_id2.unwrap();
        let key_id3 = key_id3.unwrap();

        let vault = Vault::create_with_persistent_storage(storage);
        let (attributes12, attributes22, attributes32) = join!(
            vault.get_secret_attributes(&key_id1),
            vault.get_secret_attributes(&key_id2),
            vault.get_secret_attributes(&key_id3)
        );

        // only the 2 persistent secrets can be retrieved after a restart
        assert_eq!(attributes1, attributes12.unwrap());
        assert_eq!(attributes2, attributes22.unwrap());
        assert!(attributes32.is_err());
    }
}
