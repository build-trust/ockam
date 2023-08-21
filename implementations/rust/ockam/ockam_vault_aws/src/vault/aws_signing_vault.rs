use std::path::Path;

use tracing::error;

use crate::vault::aws_kms_client::{AwsKmsClient, AwsKmsConfig, KmsClient};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, Result};
use ockam_node::{FileKeyValueStorage, InMemoryKeyValueStorage, KeyValueStorage};
use ockam_vault::{
    KeyId, PublicKey, Secret, SecretAttributes, SecretType, Signature, SigningVault, VaultError,
};

/// Security module implementation using an AWS KMS
pub struct AwsSigningVault {
    client: Arc<dyn KmsClient + Send + Sync>,
    storage: Arc<dyn KeyValueStorage<PublicKey, KeyId>>,
}

impl AwsSigningVault {
    /// Create a default AWS security module
    pub async fn default() -> Result<Self> {
        Self::new(
            AwsKmsConfig::default().await?,
            InMemoryKeyValueStorage::create(),
        )
        .await
    }

    /// Create a new AWS security module
    pub async fn new(
        config: AwsKmsConfig,
        storage: Arc<dyn KeyValueStorage<PublicKey, KeyId>>,
    ) -> Result<Self> {
        Ok(Self {
            client: Arc::new(AwsKmsClient::new(config).await?),
            storage,
        })
    }
    /// Create a new AWS security module, with a specific file storage path
    pub async fn create_with_storage_path(
        config: AwsKmsConfig,
        path: &Path,
    ) -> Result<Arc<dyn SigningVault>> {
        Self::create_with_key_value_storage(
            config,
            Arc::new(FileKeyValueStorage::create(path).await?),
        )
        .await
    }

    /// Create a new AWS security module, with a specific key value storage
    pub async fn create_with_key_value_storage(
        config: AwsKmsConfig,
        storage: Arc<dyn KeyValueStorage<PublicKey, KeyId>>,
    ) -> Result<Arc<dyn SigningVault>> {
        Ok(Arc::new(Self::new(config, storage).await?))
    }

    /// Return the key id corresponding to a public key from the KMS
    /// This function is particularly inefficient since it lists all the keys
    /// This is why there is a cache in the AwsSecurityModule struct to avoid this call
    pub(crate) async fn get_key_id_from_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
        for key_id in self.client.list_keys().await? {
            let one_public_key = self.client.public_key(&key_id).await?;
            if &one_public_key == public_key {
                return Ok(key_id);
            }
        }
        error!(%public_key, "key id not found for public key {}", public_key);
        Err(Error::new(
            Origin::Vault,
            Kind::NotFound,
            crate::vault::aws_kms_client::Error::MissingKeyId,
        ))
    }
}

#[async_trait]
impl SigningVault for AwsSigningVault {
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        let public_key = self.client.public_key(key_id).await?;

        // if the public key <-> key id mapping has not been stored locally
        // then store it in order to avoid a call to client.get_key_id when computing a identity
        // identifier from the list of identity changes
        if self.storage.get(&public_key).await?.is_none() {
            self.storage.put(public_key.clone(), key_id.clone()).await?;
        }
        Ok(public_key)
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        // try to get the key id from local storage first
        if let Some(key_id) = self.storage.get(public_key).await? {
            Ok(key_id)
        } else {
            let key_id = self.get_key_id_from_public_key(public_key).await?;
            self.storage.put(public_key.clone(), key_id.clone()).await?;
            Ok(key_id)
        }
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        self.client.sign(key_id, message).await
    }

    async fn generate_key(&self, attributes: SecretAttributes) -> Result<KeyId> {
        if attributes.secret_type() == SecretType::NistP256 {
            self.client.create_key().await
        } else {
            Err(VaultError::InvalidKeyType.into())
        }
    }

    async fn delete_key(&self, key_id: KeyId) -> Result<bool> {
        self.client.delete_key(&key_id).await
    }

    async fn import_key(&self, _key: Secret, _attributes: SecretAttributes) -> Result<KeyId> {
        unimplemented!() // FIXME: Not supported
    }

    async fn number_of_keys(&self) -> Result<usize> {
        unimplemented!() // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    use ockam_core::compat::rand::{thread_rng, RngCore};
    use ockam_node::InMemoryKeyValueStorage;
    use ockam_vault::Vault;
    use std::path::PathBuf;
    use SecretAttributes::*;

    /// This test needs to be executed with the following environment variables
    /// AWS_REGION
    /// AWS_ACCESS_KEY_ID
    /// AWS_SECRET_ACCESS_KEY
    #[tokio::test]
    #[ignore]
    async fn test_store_public_key_key_id_mapping() -> Result<()> {
        let storage = InMemoryKeyValueStorage::create();
        let security_module =
            AwsSigningVault::new(AwsKmsConfig::default().await?, storage.clone()).await?;

        let key_id = security_module.generate_key(NistP256).await?;

        // the public key can be retrieved using the kms client directly
        // but then the public key <-> key id mapping is not cached
        let public_key = security_module.client.public_key(&key_id).await;
        assert!(public_key.is_ok());
        assert!(storage.get(&public_key?).await?.is_none());

        // when the public key is retrieved using the security module
        // then the public key <-> key id mapping is cached locally
        let public_key = security_module.get_public_key(&key_id).await;
        assert!(public_key.is_ok());

        let public_key = public_key?;
        let key_id = storage.get(&public_key).await;
        assert!(key_id.is_ok());

        let key_id = security_module
            .get_key_id_from_public_key(&public_key)
            .await;
        assert!(key_id.is_ok());

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_verify() -> Result<()> {
        let security_module = AwsSigningVault::default().await?;
        let key_id = security_module.generate_key(NistP256).await?;
        let message = b"hello world";
        let signature = security_module.sign(&key_id, &message[..]).await?;
        let public_key = security_module.get_public_key(&key_id).await?;

        let verifier = Vault::create_verifying_vault();
        // Verify locally
        assert!(verifier.verify(&public_key, message, &signature).await?);

        // Verify remotely
        assert!(
            security_module
                .client
                .verify(&key_id, message, &signature)
                .await?
        );

        Ok(())
    }

    /// This test checks that the local storage mapping public keys to key ids works
    #[tokio::test]
    async fn test_storage() -> Result<()> {
        let client = Arc::new(FakeKmsClient::default());
        let storage = Arc::new(FileKeyValueStorage::create(create_temp_file().as_path()).await?);
        let security_module = AwsSigningVault { client, storage };

        let key_id = security_module.generate_key(NistP256).await?;
        let public_key = security_module.get_public_key(&key_id).await?;

        // retrieving the key id should use the mapping stored in a file
        let actual_key_id = security_module.get_key_id(&public_key).await?;

        assert_eq!(actual_key_id, key_id);

        Ok(())
    }

    // TESTS IMPLEMENTATION

    pub fn create_temp_file() -> PathBuf {
        let dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let file_name = hex::encode(bytes);
        dir.join(file_name)
    }

    struct Key(usize);

    #[derive(Default)]
    struct FakeKmsClient {
        keys: RefCell<HashMap<KeyId, Key>>,
    }

    #[allow(unsafe_code)]
    unsafe impl Send for FakeKmsClient {}

    #[allow(unsafe_code)]
    unsafe impl Sync for FakeKmsClient {}

    #[async_trait]
    impl KmsClient for FakeKmsClient {
        async fn create_key(&self) -> Result<KeyId> {
            let key = self.keys.borrow().len() + 1;
            self.keys.borrow_mut().insert(key.to_string(), Key(key));
            Ok(key.to_string())
        }

        async fn delete_key(&self, _key_id: &KeyId) -> Result<bool> {
            Ok(true)
        }

        async fn public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
            Ok(PublicKey::new(
                key_id.as_bytes().to_vec(),
                SecretType::NistP256,
            ))
        }

        /// The list_keys function returns an error to make sure that we
        /// really use the local storage to get the key id corresponding to a given public key
        async fn list_keys(&self) -> Result<Vec<KeyId>> {
            Err(Error::new(Origin::Api, Kind::Other, "can't list keys"))
        }

        async fn verify(
            &self,
            _key_id: &KeyId,
            _message: &[u8],
            _signature: &Signature,
        ) -> Result<bool> {
            Ok(true)
        }

        async fn sign(&self, _key_id: &KeyId, _message: &[u8]) -> Result<Signature> {
            Ok(Signature::new(vec![]))
        }
    }
}
