use aws_sdk_kms::error::CreateKeyError;
use aws_sdk_kms::error::GetPublicKeyError;
use aws_sdk_kms::error::SignError;
use aws_sdk_kms::error::VerifyError;
use aws_sdk_kms::model::KeySpec;
use aws_sdk_kms::model::KeyUsageType;
use aws_sdk_kms::model::MessageType;
use aws_sdk_kms::model::SigningAlgorithmSpec;
use aws_sdk_kms::types::Blob;
use aws_sdk_kms::types::SdkError;
use aws_sdk_kms::Client;
use ockam_core::async_trait;
use ockam_core::vault::SecretType;
use ockam_core::vault::{KeyId, PublicKey, Signature, Signer};
use ockam_core::Result;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// AWS KMS client.
#[derive(Debug)]
pub struct AwsKms {
    client: Client,
    multi_region: bool,
}

impl AwsKms {
    /// Create a new AWS KMS client.
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Ok(Self {
            client,
            multi_region: false,
        })
    }

    /// Configure the client to create multi-region keys.
    pub fn multi_region(mut self, val: bool) -> Self {
        self.multi_region = val;
        self
    }

    /// Create a new NIST P-256 key-pair in AWS KMS and return its ID.
    pub async fn create_key(&self) -> Result<KeyId> {
        let mut client = self
            .client
            .create_key()
            .key_usage(KeyUsageType::SignVerify)
            .key_spec(KeySpec::EccNistP256);
        if self.multi_region {
            client = client.multi_region(true)
        }
        let output = client.send().await.map_err(Error::Create)?;
        if let Some(kid) = output.key_metadata().and_then(|meta| meta.key_id()) {
            return Ok(kid.to_string());
        }
        Err(Error::MissingKeyId.into())
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
}

#[async_trait]
impl Signer for AwsKms {
    async fn sign(&self, id: &KeyId, msg: &[u8]) -> Result<Signature> {
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
    use super::AwsKms;
    use crate::Vault;
    use ockam_core::vault::{Signer, Verifier};
    use ockam_node::tokio;

    // A key ID that refers to an existing AWS KMS NIST P-256 key.
    const PREEXISTING_KEY_ID: &str = "9a573bc4-ea26-4c41-906d-532ab6d176ca";

    #[tokio::test]
    async fn sign_verify_with_existing_key() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let aws = AwsKms::new().await.unwrap();
        let msg = b"hello world";
        let sig = aws.sign(&keyid, &msg[..]).await.unwrap();
        assert!(aws.verify(&keyid, &msg[..], &sig).await.unwrap())
    }

    #[tokio::test]
    async fn sign_with_aws_verify_locally() {
        let keyid = PREEXISTING_KEY_ID.to_string();
        let aws = AwsKms::new().await.unwrap();
        let msg = b"hello world";
        let sig = aws.sign(&keyid, &msg[..]).await.unwrap();
        let pky = aws.public_key(&keyid).await.unwrap();
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
