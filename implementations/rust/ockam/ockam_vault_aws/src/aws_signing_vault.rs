use crate::aws_kms_client::{AwsKmsClient, AwsKmsConfig, KmsClient};
use crate::error::Error;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, Result};
use ockam_vault::{
    Signature, SigningKeyType, SigningSecretKeyHandle, VaultError, VaultForSigning,
    VerifyingPublicKey,
};
use tracing::error;

struct AwsKeyPair {
    key: SigningSecretKeyHandle,
    public_key: VerifyingPublicKey,
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
    pub async fn create_with_config(config: AwsKmsConfig) -> Result<Self> {
        let client = AwsKmsClient::new(config).await?;

        let mut key_pairs: Vec<AwsKeyPair> = vec![];
        // Fetch list of all keys, then fetch the public key for each key
        let keys = client.list_keys().await?;

        for key in keys {
            match client.public_key(&key).await {
                Ok(public_key) => key_pairs.push(AwsKeyPair { key, public_key }),
                // There are different possible causes here, but it's also possible that
                // the Key may in deletion pending state, or have a different key type.
                // Therefore, the best strategy is to just skip that key
                Err(err) => error!("Error exporting public key: {err}"),
            }
        }

        Ok(Self {
            client: Arc::new(client),
            keys: Arc::new(RwLock::new(key_pairs)),
        })
    }

    /// Return list of all keys
    pub fn keys(&self) -> Vec<SigningSecretKeyHandle> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .map(|x| x.key.clone())
            .collect()
    }

    /// Return number of keys
    pub async fn number_of_keys(&self) -> Result<usize> {
        Ok(self.keys.read().unwrap().len())
    }
}

#[async_trait]
impl VaultForSigning for AwsSigningVault {
    async fn sign(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
        data: &[u8],
    ) -> Result<Signature> {
        self.client.sign(signing_secret_key_handle, data).await
    }

    async fn generate_signing_secret_key(
        &self,
        signing_key_type: SigningKeyType,
    ) -> Result<SigningSecretKeyHandle> {
        if signing_key_type != SigningKeyType::ECDSASHA256CurveP256 {
            return Err(VaultError::InvalidKeyType.into());
        }

        let key = self.client.create_key().await?;
        let public_key = self.client.public_key(&key).await?;

        self.keys.write().unwrap().push(AwsKeyPair {
            key: key.clone(),
            public_key,
        });

        Ok(key)
    }

    async fn get_verifying_public_key(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<VerifyingPublicKey> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.key == signing_secret_key_handle {
                    Some(x.public_key.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn get_secret_key_handle(
        &self,
        verifying_public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.public_key == verifying_public_key {
                    Some(x.key.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn delete_signing_secret_key(
        &self,
        signing_secret_key_handle: SigningSecretKeyHandle,
    ) -> Result<bool> {
        if self.client.delete_key(&signing_secret_key_handle).await? {
            self.keys
                .write()
                .unwrap()
                .retain(|x| x.key != signing_secret_key_handle);

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
