use crate::vault::aws_kms_client::{AwsKmsClient, AwsKmsConfig};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, KeyId, Result};
use ockam_node::{FileKeyValueStorage, InMemoryKeyValueStorage, KeyValueStorage};
use ockam_vault::SecretType::NistP256;
use ockam_vault::{PublicKey, SecretAttributes, SecurityModule, Signature, VaultError};
use p256::pkcs8::DecodePublicKey;
use std::path::Path;

/// Security module implementation using an AWS KMS
pub struct AwsSecurityModule {
    client: AwsKmsClient,
    storage: Arc<dyn KeyValueStorage<PublicKey, KeyId>>,
}

impl AwsSecurityModule {
    /// Create a default AWS security module
    pub async fn default() -> Result<Self> {
        AwsSecurityModule::new(
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
        Ok(AwsSecurityModule {
            client: AwsKmsClient::new(config).await?,
            storage,
        })
    }
    /// Create a new AWS security module, with a specific file storage path
    pub async fn create_with_storage_path(
        config: AwsKmsConfig,
        path: &Path,
    ) -> Result<Arc<dyn SecurityModule>> {
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
    ) -> Result<Arc<dyn SecurityModule>> {
        Ok(Arc::new(Self::new(config, storage).await?))
    }
}

#[async_trait]
impl SecurityModule for AwsSecurityModule {
    async fn create_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        if attributes.secret_type() == NistP256 {
            self.client.create_key().await
        } else {
            Err(VaultError::InvalidKeyType.into())
        }
    }

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
            let key_id = self.client.get_key_id(public_key).await?;
            self.storage.put(public_key.clone(), key_id.clone()).await?;
            Ok(key_id)
        }
    }

    async fn get_attributes(&self, _key_id: &KeyId) -> Result<SecretAttributes> {
        Ok(SecretAttributes::NistP256)
    }

    async fn delete_secret(&self, key_id: KeyId) -> Result<bool> {
        self.client.delete_key(&key_id).await
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        self.client.sign(key_id, message).await
    }

    /// Verify the signature of a message locally
    /// This should return the same result as self.client.verify
    /// The main differences are:
    ///   - a call to self.client.verify takes more time
    ///   - a call to self.client.verify can be logged on AWS and benefit from additional access control checks
    async fn verify(
        &self,
        public_key: &PublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        use p256::ecdsa::{signature::Verifier as _, Signature, VerifyingKey};

        let verifying_key =
            VerifyingKey::from_public_key_der(public_key.data()).map_err(Self::from_pkcs8)?;
        let ecdsa_signature = Signature::from_der(signature.as_ref()).map_err(Self::from_ecdsa)?;
        Ok(verifying_key.verify(message, &ecdsa_signature).is_ok())
    }
}

impl AwsSecurityModule {
    pub(crate) fn from_ecdsa(e: p256::ecdsa::Error) -> Error {
        Error::new(Origin::Vault, Kind::Unknown, e)
    }

    pub(crate) fn from_pkcs8<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;
        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_node::InMemoryKeyValueStorage;

    /// This test needs to be executed with the following environment variables
    /// AWS_REGION
    /// AWS_ACCESS_KEY_ID
    /// AWS_SECRET_ACCESS_KEY
    #[tokio::test]
    #[ignore]
    async fn test_store_public_key_key_id_mapping() -> Result<()> {
        let storage = InMemoryKeyValueStorage::create();
        let security_module =
            AwsSecurityModule::new(AwsKmsConfig::default().await?, storage.clone()).await?;

        let key_id = security_module
            .create_secret(SecretAttributes::NistP256)
            .await?;

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

        let key_id = security_module.get_key_id(&public_key).await;
        assert!(key_id.is_ok());

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_sign_verify() -> Result<()> {
        let security_module = AwsSecurityModule::default().await?;
        let key_id = security_module
            .create_secret(SecretAttributes::NistP256)
            .await?;
        let message = b"hello world";
        let signature = security_module.sign(&key_id, &message[..]).await?;

        // Verify locally
        assert!(
            security_module
                .client
                .verify(&key_id, &message[..], &signature)
                .await?
        );

        // Verify remotely
        assert!(
            security_module
                .client
                .verify(&key_id, message, &signature)
                .await?
        );

        Ok(())
    }
}
