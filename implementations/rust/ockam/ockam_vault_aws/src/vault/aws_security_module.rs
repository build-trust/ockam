use crate::vault::aws_kms_client::{AwsKmsClient, AwsKmsConfig};
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, KeyId, Result};
use ockam_vault::SecretType::NistP256;
use ockam_vault::{PublicKey, SecretAttributes, SecurityModule, Signature, VaultError};

/// Security module implementation using an AWS KMS
pub struct AwsSecurityModule {
    client: AwsKmsClient,
}

impl AwsSecurityModule {
    /// Create a new AWS security module
    pub async fn new(config: AwsKmsConfig) -> Result<Self> {
        Ok(AwsSecurityModule {
            client: AwsKmsClient::new(config).await?,
        })
    }
    /// Create a new AWS security module
    pub async fn create(config: AwsKmsConfig) -> Result<Arc<dyn SecurityModule>> {
        Ok(Arc::new(Self::new(config).await?))
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
        self.client.public_key(key_id).await
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.client.compute_key_id(public_key).await
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

    async fn verify(
        &self,
        public_key: &PublicKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        let key_id = self.get_key_id(public_key).await?;
        self.client.verify(&key_id, message, signature).await
    }
}
