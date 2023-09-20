use crate::error::Error;
use aws_config::SdkConfig;
use aws_sdk_kms::error::SdkError;
use aws_sdk_kms::operation::schedule_key_deletion::ScheduleKeyDeletionError;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, MessageType, SigningAlgorithmSpec};
use aws_sdk_kms::Client;
use ockam_core::{async_trait, Result};
use ockam_vault::{
    ECDSASHA256CurveP256PublicKey, ECDSASHA256CurveP256Signature, HandleToSecret, Signature,
    SigningSecretKeyHandle, VerifyingPublicKey,
};
use sha2::{Digest, Sha256};
use tracing as log;

/// AWS KMS client.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    client: Client,
    config: AwsKmsConfig,
}

/// Defines how to populate the initial keys at vault startup
#[derive(Debug, Clone)]
pub enum InitialKeysDiscovery {
    /// Make a list() call to aws kms to obtain the entire set of keys this role as access to
    ListFromAwsKms,

    /// Use a specific set of key-ids
    Keys(Vec<SigningSecretKeyHandle>),
}

/// AWS KMS configuration.
#[derive(Debug, Clone)]
pub struct AwsKmsConfig {
    multi_region: bool,
    sdk_config: SdkConfig,
    initial_keys_discovery: InitialKeysDiscovery,
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
            initial_keys_discovery: InitialKeysDiscovery::ListFromAwsKms,
        }
    }

    /// Create multi-region keys.
    pub fn multi_region(mut self, val: bool) {
        self.multi_region = val;
    }

    /// Configure initial key discovery
    pub fn with_initial_keys_discovery(self, initial_keys_discovery: InitialKeysDiscovery) -> Self {
        Self {
            initial_keys_discovery,
            ..self
        }
    }
}

impl AwsKmsClient {
    /// Create a new AWS KMS client.
    pub async fn new(config: AwsKmsConfig) -> Result<AwsKmsClient> {
        let client = Client::new(&config.sdk_config);
        Ok(Self { client, config })
    }

    fn cast_handle_to_kid(handle: &SigningSecretKeyHandle) -> Result<String> {
        let handle = match handle {
            SigningSecretKeyHandle::EdDSACurve25519(_) => return Err(Error::InvalidHandle.into()),
            SigningSecretKeyHandle::ECDSASHA256CurveP256(handle) => handle.value().clone(),
        };

        let kid = String::from_utf8(handle).map_err(|_| Error::InvalidHandle)?;

        Ok(kid)
    }

    /// Create a new NIST P-256 key-pair in AWS KMS and return its ID.
    pub async fn create_key(&self) -> Result<SigningSecretKeyHandle> {
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
            let handle = SigningSecretKeyHandle::ECDSASHA256CurveP256(HandleToSecret::new(
                kid.as_bytes().to_vec(),
            ));
            return Ok(handle);
        }
        Err(Error::MissingKeyId.into())
    }

    /// Have AWS KMS schedule key deletion.
    pub async fn delete_key(&self, key: &SigningSecretKeyHandle) -> Result<bool> {
        let key = Self::cast_handle_to_kid(key)?;
        log::trace!(%key, "schedule key for deletion");
        const DAYS: i32 = 7;
        let client = self
            .client
            .schedule_key_deletion()
            .key_id(&key)
            .pending_window_in_days(DAYS);
        match client.send().await {
            Err(SdkError::ServiceError(err))
                if matches!(err.err(), ScheduleKeyDeletionError::NotFoundException(_)) =>
            {
                log::debug!(%key, "key does not exist");
                Ok(false)
            }
            Err(err) => {
                log::error!(%key, %err, "failed to schedule key for deletion");
                Err(Error::Delete {
                    keyid: key.to_string(),
                    error: err.to_string(),
                }
                .into())
            }
            Ok(_) => {
                log::debug!(%key, "key is scheduled for deletion in {DAYS} days");
                Ok(true)
            }
        }
    }

    /// Get the public key part of a AWS KMS key-pair.
    pub async fn public_key(&self, key: &SigningSecretKeyHandle) -> Result<VerifyingPublicKey> {
        let key = Self::cast_handle_to_kid(key)?;
        log::trace!(%key, "get public key");
        let output = self
            .client
            .get_public_key()
            .key_id(&key)
            .send()
            .await
            .map_err(|err| {
                log::error!(%key, %err, "failed to get public key");
                Error::Export {
                    keyid: key.to_string(),
                    error: err.to_string(),
                }
            })?;
        if output.key_spec() != Some(&KeySpec::EccNistP256) {
            log::error!(%key, "key spec not supported to get a public key");
            return Err(Error::UnsupportedKeyType.into());
        }
        if output.key_usage() != Some(&KeyUsageType::SignVerify) {
            log::error!(%key, "usage type not supported to get a public key");
            return Err(Error::UnsupportedKeyType.into());
        }
        if let Some(k) = output.public_key() {
            log::debug!(%key, "received public key");
            use p256::pkcs8::DecodePublicKey;
            let k = p256::ecdsa::VerifyingKey::from_public_key_der(k.as_ref())
                .map_err(|_| Error::InvalidPublicKeyDer)?;
            let public_key = k.to_sec1_bytes().to_vec();
            let public_key = ECDSASHA256CurveP256PublicKey(public_key.try_into().unwrap());
            return Ok(VerifyingPublicKey::ECDSASHA256CurveP256(public_key)); // FIXME
        }
        log::error!(%key, "key type not supported to get a public key");
        Err(Error::UnsupportedKeyType.into())
    }

    /// Have AWS KMS sign a message.
    pub async fn sign(&self, key: &SigningSecretKeyHandle, message: &[u8]) -> Result<Signature> {
        let key = Self::cast_handle_to_kid(key)?;
        log::trace!(%key, "sign message");
        let client = self
            .client
            .sign()
            .key_id(&key)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message(digest(message))
            .message_type(MessageType::Digest);
        let output = client.send().await.map_err(|err| {
            log::error!(%key, %err, "failed to sign message");
            Error::Sign {
                keyid: key.to_string(),
                error: err.to_string(),
            }
        })?;
        if let Some(sig) = output.signature() {
            log::debug!(%key, "signed message");
            let sig = p256::ecdsa::Signature::from_der(sig.as_ref())
                .map_err(|_| Error::InvalidSignatureDer)?;
            let sig = ECDSASHA256CurveP256Signature(sig.to_vec().try_into().unwrap()); //FIXME
            return Ok(Signature::ECDSASHA256CurveP256(sig));
        }
        log::error!(%key, "no signature received from aws");
        Err(Error::MissingSignature.into())
    }
}

/// This trait is introduced to help with the testing of the AwsSecurityModule
#[async_trait]
pub trait KmsClient {
    /// Create a key
    async fn create_key(&self) -> Result<SigningSecretKeyHandle>;

    /// Delete a key
    async fn delete_key(&self, key: &SigningSecretKeyHandle) -> Result<bool>;

    /// Get PublicKey
    async fn public_key(&self, key: &SigningSecretKeyHandle) -> Result<VerifyingPublicKey>;

    /// List All Keys
    async fn list_keys(&self) -> Result<Vec<SigningSecretKeyHandle>>;

    /// Sign a message
    async fn sign(&self, key: &SigningSecretKeyHandle, message: &[u8]) -> Result<Signature>;
}

#[async_trait]
impl KmsClient for AwsKmsClient {
    async fn create_key(&self) -> Result<SigningSecretKeyHandle> {
        self.create_key().await
    }

    async fn delete_key(&self, key_id: &SigningSecretKeyHandle) -> Result<bool> {
        self.delete_key(key_id).await
    }

    async fn list_keys(&self) -> Result<Vec<SigningSecretKeyHandle>> {
        match &self.config.initial_keys_discovery {
            InitialKeysDiscovery::ListFromAwsKms => {
                // There shouldn't be more than 2-3 active keys in the KMS,
                // however, technically we have a software limit of 100 keys here
                // If there are more keys - `list_keys` will return an Error
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
                            let key = SigningSecretKeyHandle::ECDSASHA256CurveP256(
                                HandleToSecret::new(key_id.as_bytes().to_vec()),
                            );
                            result.push(key)
                        }
                    }

                    return Ok(result);
                }

                Ok(vec![])
            }
            InitialKeysDiscovery::Keys(key_ids) => Ok(key_ids.clone()),
        }
    }

    async fn public_key(&self, key: &SigningSecretKeyHandle) -> Result<VerifyingPublicKey> {
        self.public_key(key).await
    }

    async fn sign(&self, key: &SigningSecretKeyHandle, message: &[u8]) -> Result<Signature> {
        self.sign(key, message).await
    }
}

fn digest(data: &[u8]) -> Blob {
    Blob::new(Sha256::digest(data).to_vec())
}
