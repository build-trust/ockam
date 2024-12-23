use alloc::boxed::Box;
use alloc::vec::Vec;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, Result};

use super::make_aes;
use crate::storage::SecretsRepository;
#[cfg(feature = "storage")]
use crate::storage::SecretsSqlxDatabase;

use crate::software::vault_for_secure_channels::common::generate_aead_handle;
use crate::{
    AeadSecret, AeadSecretKeyHandle, SecretBuffer, VaultError, VaultForEncryptionAtRest,
    AEAD_SECRET_LENGTH, AES_GCM_TAGSIZE, AES_NONCE_LENGTH,
};

/// [`VaultForEncryptionAtRest`] implementation using software
pub struct SoftwareVaultForAtRestEncryption {
    ephemeral_aead_secrets: Arc<RwLock<BTreeMap<AeadSecretKeyHandle, AeadSecret>>>,
    secrets_repository: Arc<dyn SecretsRepository>,
}

#[async_trait]
impl VaultForEncryptionAtRest for SoftwareVaultForAtRestEncryption {
    async fn aead_encrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        plain_text: &mut [u8],
        aad: &[u8],
    ) -> Result<()> {
        let secret = self.get_aead_secret(secret_key_handle).await?;
        let nonce = self.generate_nonce();
        let aes = make_aes(&secret);
        if plain_text.len() < AES_NONCE_LENGTH {
            return Err(VaultError::InsufficientEncryptBuffer)?;
        }
        aes.encrypt_message(&mut plain_text[AES_NONCE_LENGTH..], &nonce, aad)?;
        plain_text[..AES_NONCE_LENGTH].copy_from_slice(&nonce);
        Ok(())
    }

    async fn aead_decrypt<'a>(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        rekey_counter: u16,
        cipher_text: &'a mut [u8],
        aad: &[u8],
    ) -> Result<&'a mut [u8]> {
        let secret = self.get_aead_secret(secret_key_handle).await?;
        let secret = if rekey_counter > 0 {
            self.rekey(secret, rekey_counter).await?
        } else {
            secret
        };

        let aes = make_aes(&secret);
        // the first AES_NONCE_LENGTH bytes of the cipher text are the nonce
        let (nonce, cipher_text) = cipher_text.split_at_mut(AES_NONCE_LENGTH);
        aes.decrypt_message(cipher_text, nonce, aad)
    }

    async fn rekey_and_delete(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
    ) -> Result<AeadSecretKeyHandle> {
        let secret = self.get_aead_secret(secret_key_handle).await?;
        let new_key_secret = self.rekey(secret, 1).await?;
        let new_key_handle = self
            .import_aead_key(SecretBuffer::new(new_key_secret.0.to_vec()))
            .await?;
        self.secrets_repository
            .delete_aead_secret(secret_key_handle)
            .await?;
        self.ephemeral_aead_secrets
            .write()
            .unwrap()
            .remove(secret_key_handle);
        Ok(new_key_handle)
    }

    async fn import_aead_key(&self, secret: SecretBuffer) -> Result<AeadSecretKeyHandle> {
        if secret.data().len() != AEAD_SECRET_LENGTH {
            return Err(VaultError::InvalidSecretLength)?;
        }

        let secret = AeadSecret(<[u8; 32]>::try_from(secret.data()).unwrap());
        let handle = generate_aead_handle();

        self.secrets_repository
            .store_aead_secret(&handle, secret.clone())
            .await?;
        self.ephemeral_aead_secrets
            .write()
            .unwrap()
            .insert(handle.clone(), secret);

        Ok(handle)
    }
}

impl SoftwareVaultForAtRestEncryption {
    /// Create a new instance of [`SoftwareVaultForAtRestEncryption`]
    pub fn new(secrets_repository: Arc<dyn SecretsRepository>) -> Self {
        Self {
            ephemeral_aead_secrets: Arc::new(RwLock::new(BTreeMap::new())),
            secrets_repository,
        }
    }

    /// Create Software implementation Vault with an in-memory implementation to store secrets
    #[cfg(feature = "storage")]
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(Arc::new(
            SecretsSqlxDatabase::create().await?,
        ))))
    }

    async fn rekey(&self, secret: AeadSecret, n: u16) -> Result<AeadSecret> {
        if n == 0 {
            return Err(VaultError::InvalidRekeyCount)?;
        }

        const MAX_NONCE: [u8; AES_NONCE_LENGTH] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let mut new_key_buffer = Vec::with_capacity(32 + AES_GCM_TAGSIZE);
        let mut counter = n;
        let mut secret = secret.clone();

        while counter > 0 {
            new_key_buffer.clear();
            new_key_buffer.resize(32 + AES_GCM_TAGSIZE, 0);

            let aes = make_aes(&secret);
            aes.encrypt_message(&mut new_key_buffer, &MAX_NONCE, &[])?;
            secret = AeadSecret(<[u8; 32]>::try_from(&new_key_buffer[..32]).unwrap());

            counter -= 1;
        }

        Ok(secret)
    }

    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<AeadSecret> {
        let secret = self
            .ephemeral_aead_secrets
            .read()
            .unwrap()
            .get(handle)
            .cloned();

        match secret {
            Some(secret) => Ok(secret),
            None => {
                let secret = self.secrets_repository.get_aead_secret(handle).await?;
                let secret = secret.ok_or(VaultError::KeyNotFound)?;
                self.ephemeral_aead_secrets
                    .write()
                    .unwrap()
                    .insert(handle.clone(), secret.clone());
                Ok(secret)
            }
        }
    }

    fn generate_nonce(&self) -> [u8; AES_NONCE_LENGTH] {
        // TODO: query database for the last nonce used???
        let mut nonce = [0u8; AES_NONCE_LENGTH];
        thread_rng().fill_bytes(&mut nonce);
        nonce
    }
}
