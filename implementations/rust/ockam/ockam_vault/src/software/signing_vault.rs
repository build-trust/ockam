use crate::constants::{
    ED25519_PUBLIC_LENGTH_USIZE, ED25519_SECRET_LENGTH_U32, ED25519_SECRET_LENGTH_USIZE,
    ED25519_SIGNATURE_LENGTH_USIZE, NIST_P256_PUBLIC_LENGTH_USIZE, NIST_P256_SECRET_LENGTH_U32,
    NIST_P256_SECRET_LENGTH_USIZE, NIST_P256_SIGNATURE_LENGTH_USIZE,
};
use crate::{
    KeyId, PublicKey, Secret, SecretAttributes, SecretType, Signature, SigningVault, StoredSecret,
    VaultError,
};
use arrayref::array_ref;
use ockam_core::compat::rand::thread_rng;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Error, Result};
use ockam_node::KeyValueStorage;
use sha2::{Digest, Sha256};
use static_assertions::const_assert_eq;

const_assert_eq!(
    ed25519_dalek::SECRET_KEY_LENGTH,
    ED25519_SECRET_LENGTH_USIZE
);

const_assert_eq!(
    ed25519_dalek::PUBLIC_KEY_LENGTH,
    ED25519_PUBLIC_LENGTH_USIZE
);

const_assert_eq!(
    ed25519_dalek::SIGNATURE_LENGTH,
    ED25519_SIGNATURE_LENGTH_USIZE
);

/// [`SigningVault`] implementation using software
#[derive(Clone)]
pub struct SoftwareSigningVault {
    secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl SoftwareSigningVault {
    /// Constructor
    pub fn new(secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>) -> Self {
        Self { secrets }
    }

    /// Import a key from a binary
    pub async fn import_key(&self, key: Secret, attributes: SecretAttributes) -> Result<KeyId> {
        let public_key = Self::compute_public_key_from_secret(&key, attributes.secret_type())?;
        let key_id = Self::compute_key_id_for_public_key(&public_key)?;
        let stored_secret = StoredSecret::create(key, attributes)?;
        self.secrets.put(key_id.clone(), stored_secret).await?;
        Ok(key_id)
    }
}

impl SoftwareSigningVault {
    fn from_bytes<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;

        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }

    fn import_p256_key(key: &Secret) -> Result<p256::ecdsa::SigningKey> {
        if key.as_ref().len() != NIST_P256_SECRET_LENGTH_USIZE {
            return Err(VaultError::InvalidSecretLength(
                SecretType::NistP256,
                key.as_ref().len(),
                NIST_P256_SECRET_LENGTH_U32,
            )
            .into());
        }

        p256::ecdsa::SigningKey::from_bytes(key.as_ref().into()).map_err(Self::from_bytes)
    }

    fn import_ed25519_key(key: &Secret) -> Result<ed25519_dalek::SigningKey> {
        if key.as_ref().len() != ED25519_SECRET_LENGTH_USIZE {
            return Err(VaultError::InvalidSecretLength(
                SecretType::Ed25519,
                key.as_ref().len(),
                ED25519_SECRET_LENGTH_U32,
            )
            .into());
        }

        let secret = array_ref![key.as_ref(), 0, ED25519_SECRET_LENGTH_USIZE];

        Ok(ed25519_dalek::SigningKey::from_bytes(secret))
    }

    fn compute_public_key_from_secret(key: &Secret, stype: SecretType) -> Result<PublicKey> {
        match stype {
            SecretType::Ed25519 => {
                let signing_key = Self::import_ed25519_key(key)?;
                let verifying_key = signing_key.verifying_key();
                let verifying_key = verifying_key.to_bytes().to_vec();

                if verifying_key.len() != ED25519_PUBLIC_LENGTH_USIZE {
                    return Err(VaultError::InvalidPublicLength.into());
                }

                Ok(PublicKey::new(verifying_key, SecretType::Ed25519))
            }
            SecretType::NistP256 => {
                let signing_key = Self::import_p256_key(key)?;
                let verifying_key = signing_key.verifying_key();
                let verifying_key = verifying_key.to_sec1_bytes().to_vec();

                if verifying_key.len() != NIST_P256_PUBLIC_LENGTH_USIZE {
                    return Err(VaultError::InvalidPublicLength.into());
                }

                Ok(PublicKey::new(verifying_key, SecretType::NistP256))
            }
            SecretType::X25519 | SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }

    fn compute_key_id_for_public_key(public_key: &PublicKey) -> Result<KeyId> {
        let digest = Sha256::digest(public_key.data());
        Ok(hex::encode(digest))
    }

    async fn get_key(&self, key_id: &KeyId) -> Result<StoredSecret> {
        let stored_secret = self
            .secrets
            .get(key_id)
            .await?
            .ok_or(VaultError::KeyNotFound)?;
        Ok(stored_secret)
    }
}

#[async_trait]
impl SigningVault for SoftwareSigningVault {
    async fn generate_key(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let key = match attributes.secret_type() {
            SecretType::Ed25519 => {
                // Just random 32 bytes
                let signing_key = ed25519_dalek::SigningKey::generate(&mut thread_rng());
                let signing_key = signing_key.to_bytes().to_vec();

                if signing_key.len() != ED25519_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidSecretLength(
                        SecretType::Ed25519,
                        signing_key.len(),
                        ED25519_SECRET_LENGTH_U32,
                    )
                    .into());
                }

                Secret::new(signing_key)
            }
            SecretType::NistP256 => {
                // Somewhat special random 32 bytes
                let signing_key = p256::ecdsa::SigningKey::random(&mut thread_rng());
                let signing_key = signing_key.to_bytes().to_vec();

                if signing_key.len() != NIST_P256_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidSecretLength(
                        SecretType::NistP256,
                        signing_key.len(),
                        NIST_P256_SECRET_LENGTH_U32,
                    )
                    .into());
                }

                Secret::new(signing_key)
            }
            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                return Err(VaultError::InvalidKeyType.into());
            }
        };

        let key_id = self.import_key(key, attributes).await?;

        Ok(key_id)
    }

    async fn delete_key(&self, key_id: KeyId) -> Result<bool> {
        self.secrets.delete(&key_id).await.map(|r| r.is_some())
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        let secret = self.get_key(key_id).await?;

        Self::compute_public_key_from_secret(secret.secret(), secret.attributes().secret_type())
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        Self::compute_key_id_for_public_key(public_key)
    }

    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        let stored_secret = self.get_key(key_id).await?;

        match stored_secret.attributes().secret_type() {
            SecretType::Ed25519 => {
                use ed25519_dalek::Signer;
                let key = Self::import_ed25519_key(stored_secret.secret())?;
                let signature = key.sign(data).to_vec();

                if signature.len() != ED25519_SIGNATURE_LENGTH_USIZE {
                    return Err(VaultError::InvalidSignatureSize.into());
                }

                Ok(Signature::new(signature))
            }
            SecretType::NistP256 => {
                use p256::ecdsa::signature::Signer;
                let key = Self::import_p256_key(stored_secret.secret())?;
                let signature: p256::ecdsa::Signature = key.sign(data);
                let signature = signature.to_vec();

                if signature.len() != NIST_P256_SIGNATURE_LENGTH_USIZE {
                    return Err(VaultError::InvalidSignatureSize.into());
                }

                Ok(Signature::new(signature))
            }
            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }

    async fn number_of_keys(&self) -> Result<usize> {
        Ok(self.secrets.keys().await?.len())
    }
}
