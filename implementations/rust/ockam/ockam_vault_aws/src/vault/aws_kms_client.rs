use aws_sdk_kms::error::SdkError;
use aws_sdk_kms::operation::create_key::CreateKeyError;
use aws_sdk_kms::operation::get_public_key::GetPublicKeyError;
use aws_sdk_kms::operation::schedule_key_deletion::ScheduleKeyDeletionError;
use aws_sdk_kms::operation::sign::SignError;
use aws_sdk_kms::operation::verify::VerifyError;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{KeySpec, KeyUsageType, MessageType, SigningAlgorithmSpec};
use aws_sdk_kms::Client;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{KeyId, Result};
use ockam_vault::{PublicKey, SecretType, Signature};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing as log;

/// AWS KMS client.
#[derive(Debug, Clone)]
pub struct AwsKmsClient {
    client: Client,
    config: AwsKmsConfig,
}

/// AWS KMS configuration.
#[derive(Debug, Default, Clone)]
pub struct AwsKmsConfig {
    multi_region: bool,
}

impl AwsKmsConfig {
    /// Create a new configuration for the AWS KMS
    pub fn new() -> AwsKmsConfig {
        AwsKmsConfig {
            multi_region: false,
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
    pub async fn new(c: AwsKmsConfig) -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Ok(Self { client, config: c })
    }

    /// Create an AWS KMS client using the default configuration.
    pub async fn default() -> Result<Self> {
        Self::new(AwsKmsConfig::default()).await
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
                return Err(Error::Create(err).into());
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
                    error: err,
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
                    error: err,
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
            return Ok(PublicKey::new(k.as_ref().to_vec(), SecretType::NistP256));
        }
        log::error!(%key_id, "key type not supported to get a public key");
        Err(Error::UnsupportedKeyType.into())
    }

    pub(crate) async fn compute_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        log::trace!(%public_key, "compute the key id for a public key");
        let output = self
            .client
            .describe_key()
            .key_id(public_key.to_string())
            .send()
            .await
            .map_err(|err| {
                log::error!(%public_key, %err, "failed to get the key description");
                Error::MissingKeyId
            })?;
        if let Some(metadata) = output.key_metadata() {
            log::debug!(%public_key, "received key id");
            if let Some(key_id) = metadata.key_id() {
                return Ok(key_id.to_string());
            }
        }
        log::error!(%public_key, "key id not found for public key {}", public_key);
        Err(ockam_core::Error::new(
            Origin::Vault,
            Kind::NotFound,
            Error::MissingKeyId,
        ))
    }

    /// Have AWS KMS verify a message signature.
    pub async fn verify(
        &self,
        key_id: &KeyId,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        log::trace!(%key_id, "verify message signature");
        let client = self
            .client
            .verify()
            .key_id(key_id)
            .signature(Blob::new(signature.as_ref()))
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message(digest(message))
            .message_type(MessageType::Digest);
        let output = client.send().await.map_err(|err| {
            log::error!(%key_id, %err, "failed to verify message signature");
            Error::Verify {
                keyid: key_id.to_string(),
                error: err,
            }
        })?;
        let is_valid = output.signature_valid();
        log::debug!(%key_id, %is_valid, "verified message signature");
        Ok(is_valid)
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
                error: err,
            }
        })?;
        if let Some(sig) = output.signature() {
            log::debug!(%key_id, "signed message");
            return Ok(Signature::new(sig.as_ref().to_vec()));
        }
        log::error!(%key_id, "no signature received from aws");
        Err(Error::MissingSignature.into())
    }
}

fn digest(data: &[u8]) -> Blob {
    Blob::new(Sha256::digest(data).to_vec())
}

#[derive(Error, Debug)]
enum Error {
    #[error("aws sdk error creating new key")]
    Create(#[from] SdkError<CreateKeyError>),
    #[error("aws sdk error signing message with key {keyid}")]
    Sign {
        keyid: String,
        #[source]
        error: SdkError<SignError>,
    },
    #[error("aws sdk error verifying message with key {keyid}")]
    Verify {
        keyid: String,
        #[source]
        error: SdkError<VerifyError>,
    },
    #[error("aws sdk error exporting public key {keyid}")]
    Export {
        keyid: String,
        #[source]
        error: SdkError<GetPublicKeyError>,
    },
    #[error("aws sdk error exporting public key {keyid}")]
    Delete {
        keyid: String,
        #[source]
        error: SdkError<ScheduleKeyDeletionError>,
    },
    #[error("aws did not return a key id")]
    MissingKeyId,
    #[error("aws did not return a signature")]
    MissingSignature,
    #[error("key type is not supported")]
    UnsupportedKeyType,
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> Self {
        ockam_core::Error::new(Origin::Other, Kind::Io, e)
    }
}

#[cfg(test)]
mod tests {
    use super::AwsKmsClient;
    use ockam_node::tokio;
    use ockam_vault::{SecretAttributes, SecretsStore, Signer, Vault};

    // A key ID that refers to an existing AWS KMS NIST P-256 key.
    const PREEXISTING_KEY_ID: &str = "d1583be3-23f6-4ad7-9214-33a1e64e2374";

    #[tokio::test]
    #[ignore]
    async fn sign_verify_with_existing_key() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let kms = AwsKmsClient::default().await.unwrap();
        let msg = b"hello world";
        let sig = kms.sign(&keyid, &msg[..]).await.unwrap();
        assert!(kms.verify(&keyid, &msg[..], &sig).await.unwrap())
    }

    #[tokio::test]
    #[ignore]
    async fn sign_with_aws_verify_locally() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let kms = AwsKmsClient::default().await.unwrap();
        let msg = b"hello world";
        let sig = kms.sign(&keyid, &msg[..]).await.unwrap();
        let pky = kms.public_key(&keyid).await.unwrap();
        let vault = Vault::create();
        {
            let att = SecretAttributes::NistP256;
            let kid = vault.create_ephemeral_secret(att).await.unwrap();
            let pky = vault.get_public_key(&kid).await.unwrap();
            let sig = vault.sign(&kid, &msg[..]).await.unwrap();
            assert!(vault.verify(&pky, msg, &sig).await.unwrap())
        }
        assert!(vault.verify(&pky, msg, &sig).await.unwrap())
    }
}
