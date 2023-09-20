use crate::{
    ECDSASHA256CurveP256PublicKey, ECDSASHA256CurveP256SecretKey, ECDSASHA256CurveP256Signature,
    EdDSACurve25519PublicKey, EdDSACurve25519SecretKey, EdDSACurve25519Signature, HandleToSecret,
    Signature, SigningKeyType, SigningSecret, SigningSecretKeyHandle, VaultError, VaultForSigning,
    VerifyingPublicKey,
};
use crate::{
    ECDSA_SHA256_CURVEP256_PUBLIC_KEY_LENGTH, ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH,
    EDDSA_CURVE25519_SECRET_KEY_LENGTH,
};

use ockam_core::compat::rand::thread_rng;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Error, Result};
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};

use crate::legacy::KeyId;
use crate::software::legacy::StoredSecret;
use arrayref::array_ref;
use sha2::{Digest, Sha256};

/// [`SigningVault`] implementation using software
#[derive(Clone)]
pub struct SoftwareVaultForSigning {
    // Use String as a key for backwards compatibility
    secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl SoftwareVaultForSigning {
    /// Constructor
    pub fn new(secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>) -> Self {
        Self { secrets }
    }

    /// Create Software implementation Vault with [`InMemoryKeyVaultStorage`]
    pub fn create() -> Arc<SoftwareVaultForSigning> {
        Arc::new(Self::new(InMemoryKeyValueStorage::create()))
    }

    /// Import a key from a binary
    pub async fn import_key(&self, key: SigningSecret) -> Result<SigningSecretKeyHandle> {
        let public_key = Self::compute_public_key_from_secret(&key)?;
        let handle = Self::compute_handle_for_public_key(&public_key)?;

        self.secrets
            .put(hex::encode(handle.handle().value()), key.into())
            .await?;

        Ok(handle)
    }

    /// Return the total number of keys
    pub async fn number_of_keys(&self) -> Result<usize> {
        Ok(self.secrets.keys().await?.len())
    }
}

#[async_trait]
impl VaultForSigning for SoftwareVaultForSigning {
    async fn sign(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
        data: &[u8],
    ) -> Result<Signature> {
        let signing_secret = self.get_stored_secret(signing_secret_key_handle).await?;

        match signing_secret {
            SigningSecret::EdDSACurve25519(secret) => {
                use ed25519_dalek::Signer;
                let key = Self::import_ed25519_key(secret.key())?;
                let signature = key.sign(data).to_bytes();

                let signature = EdDSACurve25519Signature(signature);
                let signature = Signature::EdDSACurve25519(signature);

                Ok(signature)
            }
            SigningSecret::ECDSASHA256CurveP256(secret) => {
                use p256::ecdsa::signature::Signer;
                let key = Self::import_p256_key(secret.key())?;
                let signature: p256::ecdsa::Signature = key.sign(data);
                let signature = signature.to_bytes();

                let signature = ECDSASHA256CurveP256Signature(signature.into());
                let signature = Signature::ECDSASHA256CurveP256(signature);

                Ok(signature)
            }
        }
    }

    async fn generate_signing_secret_key(
        &self,
        signing_key_type: SigningKeyType,
    ) -> Result<SigningSecretKeyHandle> {
        let key = match signing_key_type {
            SigningKeyType::EdDSACurve25519 => {
                // Just random 32 bytes
                let signing_key = ed25519_dalek::SigningKey::generate(&mut thread_rng());
                let signing_key = signing_key.to_bytes();
                let signing_key = EdDSACurve25519SecretKey::new(signing_key);

                SigningSecret::EdDSACurve25519(signing_key)
            }
            SigningKeyType::ECDSASHA256CurveP256 => {
                // Somewhat special random 32 bytes
                let signing_key = p256::ecdsa::SigningKey::random(&mut thread_rng());
                let signing_key = signing_key.to_bytes();
                let signing_key = ECDSASHA256CurveP256SecretKey::new(signing_key.into());

                SigningSecret::ECDSASHA256CurveP256(signing_key)
            }
        };

        let handle = self.import_key(key).await?;

        Ok(handle)
    }

    async fn get_verifying_public_key(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<VerifyingPublicKey> {
        let secret = self.get_stored_secret(signing_secret_key_handle).await?;

        Self::compute_public_key_from_secret(&secret)
    }

    async fn get_secret_key_handle(
        &self,
        verifying_public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle> {
        Self::compute_handle_for_public_key(verifying_public_key)
    }

    async fn delete_signing_secret_key(
        &self,
        signing_secret_key_handle: SigningSecretKeyHandle,
    ) -> Result<bool> {
        self.secrets
            .delete(&hex::encode(signing_secret_key_handle.handle().value()))
            .await
            .map(|r| r.is_some())
    }
}

impl SoftwareVaultForSigning {
    fn from_bytes<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;

        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }

    fn import_p256_key(
        key: &[u8; ECDSA_SHA256_CURVEP256_SECRET_KEY_LENGTH],
    ) -> Result<p256::ecdsa::SigningKey> {
        p256::ecdsa::SigningKey::from_bytes(key.as_ref().into()).map_err(Self::from_bytes)
    }

    fn import_ed25519_key(
        key: &[u8; EDDSA_CURVE25519_SECRET_KEY_LENGTH],
    ) -> Result<ed25519_dalek::SigningKey> {
        Ok(ed25519_dalek::SigningKey::from_bytes(key))
    }

    fn compute_public_key_from_secret(key: &SigningSecret) -> Result<VerifyingPublicKey> {
        match key {
            SigningSecret::EdDSACurve25519(key) => {
                let signing_key = Self::import_ed25519_key(key.key())?;
                let verifying_key = signing_key.verifying_key();
                let verifying_key = verifying_key.to_bytes();

                let verifying_key = EdDSACurve25519PublicKey(verifying_key);
                let verifying_key = VerifyingPublicKey::EdDSACurve25519(verifying_key);

                Ok(verifying_key)
            }
            SigningSecret::ECDSASHA256CurveP256(key) => {
                let signing_key = Self::import_p256_key(key.key())?;
                let verifying_key = signing_key.verifying_key();
                let verifying_key = verifying_key.to_sec1_bytes().to_vec();

                if verifying_key.len() != ECDSA_SHA256_CURVEP256_PUBLIC_KEY_LENGTH {
                    return Err(VaultError::InvalidPublicLength.into());
                }

                let verifying_key =
                    *array_ref![verifying_key, 0, ECDSA_SHA256_CURVEP256_PUBLIC_KEY_LENGTH];
                let verifying_key = ECDSASHA256CurveP256PublicKey(verifying_key);
                let verifying_key = VerifyingPublicKey::ECDSASHA256CurveP256(verifying_key);

                Ok(verifying_key)
            }
        }
    }

    fn compute_handle_for_public_key(
        public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle> {
        let handle = match public_key {
            VerifyingPublicKey::EdDSACurve25519(public_key) => {
                let digest = Sha256::digest(public_key.0);
                let handle = HandleToSecret::new(digest.to_vec());
                SigningSecretKeyHandle::EdDSACurve25519(handle)
            }
            VerifyingPublicKey::ECDSASHA256CurveP256(public_key) => {
                let digest = Sha256::digest(public_key.0);
                let handle = HandleToSecret::new(digest.to_vec());
                SigningSecretKeyHandle::ECDSASHA256CurveP256(handle)
            }
        };

        Ok(handle)
    }

    async fn get_stored_secret(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<SigningSecret> {
        let handle = match signing_secret_key_handle {
            SigningSecretKeyHandle::EdDSACurve25519(handle) => handle,
            SigningSecretKeyHandle::ECDSASHA256CurveP256(handle) => handle,
        };

        let stored_secret = self
            .secrets
            .get(&hex::encode(handle.value()))
            .await?
            .ok_or(VaultError::KeyNotFound)?;

        stored_secret.try_into()
    }
}
