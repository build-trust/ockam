use crate::error::Error;
use aws_config::SdkConfig;
use aws_sdk_kms::error::SdkError;
use aws_sdk_kms::operation::schedule_key_deletion::ScheduleKeyDeletionError;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, MessageType, SigningAlgorithmSpec};
use aws_sdk_kms::Client;
use ockam_core::{async_trait, Result};
use ockam_vault::{KeyId, PublicKey, SecretType, Signature};
use sha2::{Digest, Sha256};
use tracing as log;

/// AWS KMS client.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    client: Client,
    config: AwsKmsConfig,
}

/// AWS KMS configuration.
#[derive(Debug, Clone)]
pub struct AwsKmsConfig {
    multi_region: bool,
    sdk_config: SdkConfig,
}

impl AwsKmsConfig {
    /// Create a new configuration for the AWS KMS
    pub async fn default() -> Result<AwsKmsConfig> {
        Ok(Self::new(aws_config::load_from_env().await))
    }

    /// Create a new configuration for the AWS KMS
    pub fn new(sdk_config: SdkConfig) -> AwsKmsConfig {
        AwsKmsConfig {
            multi_region: false,
            sdk_config,
        }
    }

    /// Create multi-region keys.
    pub fn multi_region(mut self, val: bool) -> Self {
        self.multi_region = val;
        self
    }
}

impl AwsKmsClient {
    /// Create a new AWS KMS client.
    pub async fn new(config: AwsKmsConfig) -> Result<AwsKmsClient> {
        let client = Client::new(&config.sdk_config);
        Ok(Self { client, config })
    }

    /// Create a new NIST P-256 key-pair in AWS KMS and return its ID.
    pub async fn create_key(&self) -> Result<KeyId> {
        log::trace!("create new key");
        let mut client = self
            .client
            .create_key()
            .key_usage(KeyUsageType::SignVerify)
            .key_spec(KeySpec::EccNistP256);
        if self.config.multi_region {
            client = client.multi_region(true)
        }
        let output = match client.send().await {
            Ok(out) => out,
            Err(err) => {
                log::error!(%err, "failed to create new key");
                return Err(Into::<ockam_core::Error>::into(Error::Create(
                    err.to_string(),
                )));
            }
        };
        if let Some(kid) = output.key_metadata().and_then(|meta| meta.key_id()) {
            log::debug!(%kid, "created new key");
            return Ok(kid.to_string());
        }
        Err(Error::MissingKeyId.into())
    }

    /// Have AWS KMS schedule key deletion.
    pub async fn delete_key(&self, key_id: &KeyId) -> Result<bool> {
        log::trace!(%key_id, "schedule key for deletion");
        const DAYS: i32 = 7;
        let client = self
            .client
            .schedule_key_deletion()
            .key_id(key_id)
            .pending_window_in_days(DAYS);
        match client.send().await {
            Err(SdkError::ServiceError(err))
                if matches!(err.err(), ScheduleKeyDeletionError::NotFoundException(_)) =>
            {
                log::debug!(%key_id, "key does not exist");
                Ok(false)
            }
            Err(err) => {
                log::error!(%key_id, %err, "failed to schedule key for deletion");
                Err(Error::Delete {
                    keyid: key_id.to_string(),
                    error: err.to_string(),
                }
                .into())
            }
            Ok(_) => {
                log::debug!(%key_id, "key is scheduled for deletion in {DAYS} days");
                Ok(true)
            }
        }
    }

    /// Get the public key part of a AWS KMS key-pair.
    pub async fn public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        log::trace!(%key_id, "get public key");
        let output = self
            .client
            .get_public_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|err| {
                log::error!(%key_id, %err, "failed to get public key");
                Error::Export {
                    keyid: key_id.to_string(),
                    error: err.to_string(),
                }
            })?;
        if output.key_spec() != Some(&KeySpec::EccNistP256) {
            log::error!(%key_id, "key spec not supported to get a public key");
            return Err(Error::UnsupportedKeyType.into());
        }
        if output.key_usage() != Some(&KeyUsageType::SignVerify) {
            log::error!(%key_id, "usage type not supported to get a public key");
            return Err(Error::UnsupportedKeyType.into());
        }
        if let Some(k) = output.public_key() {
            log::debug!(%key_id, "received public key");
            use p256::pkcs8::DecodePublicKey;
            let k = p256::ecdsa::VerifyingKey::from_public_key_der(k.as_ref())
                .map_err(|_| Error::InvalidPublicKeyDer)?;
            return Ok(PublicKey::new(
                k.to_sec1_bytes().to_vec(),
                SecretType::NistP256,
            ));
        }
        log::error!(%key_id, "key type not supported to get a public key");
        Err(Error::UnsupportedKeyType.into())
    }

    /// Have AWS KMS sign a message.
    pub async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        log::trace!(%key_id, "sign message");
        let client = self
            .client
            .sign()
            .key_id(key_id)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message(digest(message))
            .message_type(MessageType::Digest);
        let output = client.send().await.map_err(|err| {
            log::error!(%key_id, %err, "failed to sign message");
            Error::Sign {
                keyid: key_id.to_string(),
                error: err.to_string(),
            }
        })?;
        if let Some(sig) = output.signature() {
            log::debug!(%key_id, "signed message");
            let sig = p256::ecdsa::Signature::from_der(sig.as_ref())
                .map_err(|_| Error::InvalidSignatureDer)?;
            return Ok(Signature::new(sig.to_vec()));
        }
        log::error!(%key_id, "no signature received from aws");
        Err(Error::MissingSignature.into())
    }
}

/// This trait is introduced to help with the testing of the AwsSecurityModule
#[async_trait]
pub trait KmsClient {
    /// Create a key
    async fn create_key(&self) -> Result<KeyId>;

    /// Delete a key
    async fn delete_key(&self, key_id: &KeyId) -> Result<bool>;

    /// Get PublicKey
    async fn public_key(&self, key_id: &KeyId) -> Result<PublicKey>;

    /// List All Keys
    async fn list_keys(&self) -> Result<Vec<KeyId>>;

    /// Sign a message
    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature>;
}

#[async_trait]
impl KmsClient for AwsKmsClient {
    async fn create_key(&self) -> Result<KeyId> {
        self.create_key().await
    }

    async fn delete_key(&self, key_id: &KeyId) -> Result<bool> {
        self.delete_key(key_id).await
    }

    async fn list_keys(&self) -> Result<Vec<KeyId>> {
        let output = self.client.list_keys().send().await.map_err(|err| {
            log::error!(%err, "failed to list all keys");
            Error::MissingKeys
        })?;

        if output.truncated() {
            return Err(Error::TruncatedKeysList.into());
        }

        if let Some(keys) = output.keys() {
            let mut result = vec![];
            for key in keys {
                if let Some(key_id) = key.key_id() {
                    result.push(key_id.to_string())
                }
            }
            return Ok(result);
        }

        Ok(vec![])
    }

    async fn public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.public_key(key_id).await
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        self.sign(key_id, message).await
    }
}

fn digest(data: &[u8]) -> Blob {
    Blob::new(Sha256::digest(data).to_vec())
}
