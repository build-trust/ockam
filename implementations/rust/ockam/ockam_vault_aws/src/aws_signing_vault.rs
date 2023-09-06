use crate::aws_kms_client::{AwsKmsClient, AwsKmsConfig, KmsClient};
use crate::error::Error;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, Result};
use ockam_vault::{
    KeyId, PublicKey, SecretAttributes, SecretType, Signature, SigningVault, VaultError,
};
use tracing::error;

struct AwsKeyPair {
    key_id: KeyId,
    public_key: PublicKey,
}

/// Security module implementation using an AWS KMS
pub struct AwsSigningVault {
    client: Arc<dyn KmsClient + Send + Sync>,
    // Store mapping from PublicKey to KeyId in memory
    // This is fetched at the Vault initialization
    // and is updated locally during add/delete operations
    // WARNING: The assumption is that there is no concurrent access to the same keys from
    // different places.
    keys: Arc<RwLock<Vec<AwsKeyPair>>>,
}

impl AwsSigningVault {
    /// Create a default AWS security module
    pub async fn create() -> Result<Self> {
        Self::create_with_config(AwsKmsConfig::default().await?).await
    }

    /// Create a new AWS security module
    async fn create_with_config(config: AwsKmsConfig) -> Result<Self> {
        let client = AwsKmsClient::new(config).await?;

        let mut keys: Vec<AwsKeyPair> = vec![];

        // Fetch list of all keys, then fetch the public key for each key
        // There shouldn't be more than 2-3 active keys in the KMS,
        // however, technically we have a software limit of 100 keys here
        // If there are more keys - `list_keys` will return an Error
        // TODO: Make sure every Vault in AWS account gets its isolated scope
        let key_ids = client.list_keys().await?;

        for key_id in key_ids {
            match client.public_key(&key_id).await {
                Ok(public_key) => keys.push(AwsKeyPair { key_id, public_key }),
                // There are different possible causes here, but it's also possible that
                // the Key may in deletion pending state, or have a different key type.
                // Therefore, the best strategy is to just skip that key
                Err(err) => error!("Error exporting public key: {err}"),
            }
        }

        Ok(Self {
            client: Arc::new(client),
            keys: Arc::new(RwLock::new(keys)),
        })
    }

    /// Return list of all keys
    pub fn keys(&self) -> Vec<KeyId> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .map(|x| x.key_id.clone())
            .collect()
    }
}

#[async_trait]
impl SigningVault for AwsSigningVault {
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.key_id == key_id {
                    Some(x.public_key.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.public_key == public_key {
                    Some(x.key_id.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        self.client.sign(key_id, message).await
    }

    async fn generate_key(&self, attributes: SecretAttributes) -> Result<KeyId> {
        if attributes.secret_type() != SecretType::NistP256 {
            return Err(VaultError::InvalidKeyType.into());
        }

        let key_id = self.client.create_key().await?;
        let public_key = self.client.public_key(&key_id).await?;

        self.keys.write().unwrap().push(AwsKeyPair {
            key_id: key_id.clone(),
            public_key,
        });

        Ok(key_id)
    }

    async fn delete_key(&self, key_id: KeyId) -> Result<bool> {
        if self.client.delete_key(&key_id).await? {
            self.keys.write().unwrap().retain(|x| x.key_id != key_id);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn number_of_keys(&self) -> Result<usize> {
        Ok(self.keys.read().unwrap().len())
    }
}
