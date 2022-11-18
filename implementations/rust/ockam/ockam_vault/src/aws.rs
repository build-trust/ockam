use aws_sdk_kms::error::{SignError, VerifyError, GetPublicKeyError, CreateKeyError};
use aws_sdk_kms::error::{ScheduleKeyDeletionError, ScheduleKeyDeletionErrorKind};
use aws_sdk_kms::model::{KeySpec, KeyUsageType, MessageType, SigningAlgorithmSpec};
use aws_sdk_kms::types::{Blob, SdkError};
use aws_sdk_kms::Client;
use ockam_core::vault::SecretType;
use ockam_core::vault::{KeyId, PublicKey, Signature};
use ockam_core::Result;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// AWS KMS configuration.
#[derive(Debug, Default, Clone)]
pub struct Config {
    multi_region: bool,
}

impl Config {
    /// Create multi-region keys.
    pub fn multi_region(mut self, val: bool) -> Self {
        self.multi_region = val;
        self
    }
}


/// AWS KMS client.
#[derive(Debug, Clone)]
pub struct Kms {
    client: Client,
    config: Config
}

impl Kms {
    /// Create a new AWS KMS client.
    pub async fn new(c: Config) -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Ok(Self {
            client,
            config: c
        })
    }

    /// Create an AWS KMS client using the default configutation.
    pub async fn default() -> Result<Self> {
        Self::new(Config::default()).await
    }

    /// Create a new NIST P-256 key-pair in AWS KMS and return its ID.
    pub async fn create_key(&self) -> Result<KeyId> {
        let mut client = self
            .client
            .create_key()
            .key_usage(KeyUsageType::SignVerify)
            .key_spec(KeySpec::EccNistP256);
        if self.config.multi_region {
            client = client.multi_region(true)
        }
        let output = client.send().await.map_err(Error::Create)?;
        if let Some(kid) = output.key_metadata().and_then(|meta| meta.key_id()) {
            return Ok(kid.to_string());
        }
        Err(Error::MissingKeyId.into())
    }

    /// Have AWS KMS schedule key deletion.
    pub async fn delete_key(&self, id: &KeyId) -> Result<bool> {
        let client = self
            .client
            .schedule_key_deletion()
            .key_id(id)
            .pending_window_in_days(7);
        match client.send().await {
            Err(SdkError::ServiceError { err, .. })
                if matches!(err.kind, ScheduleKeyDeletionErrorKind::NotFoundException(_)) => {
                    Ok(false)
                }
            Err(e) => Err(Error::Delete { keyid: id.to_string(), error: e }.into()),
            Ok(_)  => Ok(true)
        }
    }

    /// Get the public key part of a AWS KMS key-pair.
    pub async fn public_key(&self, id: &KeyId) -> Result<PublicKey> {
        let output = self
            .client
            .get_public_key()
            .key_id(id)
            .send()
            .await
            .map_err(|e| Error::Export {
                keyid: id.to_string(),
                error: e,
            })?;
        if output.key_spec() != Some(&KeySpec::EccNistP256) {
            return Err(Error::UnsupportedKeyType.into());
        }
        if output.key_usage() != Some(&KeyUsageType::SignVerify) {
            return Err(Error::UnsupportedKeyType.into());
        }
        if let Some(k) = output.public_key() {
            return Ok(PublicKey::new(k.as_ref().to_vec(), SecretType::NistP256));
        }
        Err(Error::UnsupportedKeyType.into())
    }

    /// Have AWS KMS verify a message signature.
    pub async fn verify(&self, id: &KeyId, msg: &[u8], sig: &Signature) -> Result<bool> {
        let client = self
            .client
            .verify()
            .key_id(id)
            .signature(Blob::new(sig.as_ref()))
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message(digest(msg))
            .message_type(MessageType::Digest);
        let output = client.send().await.map_err(|e| Error::Verify {
            keyid: id.to_string(),
            error: e,
        })?;
        Ok(output.signature_valid())
    }

    /// Have AWS KMS sign a message.
    pub async fn sign(&self, id: &KeyId, msg: &[u8]) -> Result<Signature> {
        let client = self
            .client
            .sign()
            .key_id(id)
            .signing_algorithm(SigningAlgorithmSpec::EcdsaSha256)
            .message(digest(msg))
            .message_type(MessageType::Digest);
        let output = client.send().await.map_err(|e| Error::Sign {
            keyid: id.to_string(),
            error: e,
        })?;
        if let Some(sig) = output.signature() {
            return Ok(Signature::new(sig.as_ref().to_vec()));
        }
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
        use ockam_core::errcode::{Kind, Origin};
        ockam_core::Error::new(Origin::Other, Kind::Io, e)
    }
}

#[cfg(test)]
mod tests {
    use super::Kms;
    use crate::Vault;
    use ockam_core::vault::{Signer, Verifier};
    use ockam_node::tokio;

    // A key ID that refers to an existing AWS KMS NIST P-256 key.
    const PREEXISTING_KEY_ID: &str = "9a573bc4-ea26-4c41-906d-532ab6d176ca";

    #[tokio::test]
    async fn sign_verify_with_existing_key() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let kms = Kms::new().await.unwrap();
        let msg = b"hello world";
        let sig = kms.sign(&keyid, &msg[..]).await.unwrap();
        assert!(kms.verify(&keyid, &msg[..], &sig).await.unwrap())
    }

    #[tokio::test]
    async fn sign_with_aws_verify_locally() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let kms = Kms::new().await.unwrap();
        let msg = b"hello world";
        let sig = kms.sign(&keyid, &msg[..]).await.unwrap();
        let pky = kms.public_key(&keyid).await.unwrap();
        let vlt = Vault::create();
        {
            use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, SecretVault};
            let att = SecretAttributes::new(SecretType::NistP256, SecretPersistence::Ephemeral, 32);
            let kid = vlt.secret_generate(att).await.unwrap();
            let pky = vlt.secret_public_key_get(&kid).await.unwrap();
            let sig = vlt.sign(&kid, &msg[..]).await.unwrap();
            assert!(vlt.verify(&sig, &pky, msg).await.unwrap())
        }
        assert!(vlt.verify(&sig, &pky, msg).await.unwrap())
    }
}
