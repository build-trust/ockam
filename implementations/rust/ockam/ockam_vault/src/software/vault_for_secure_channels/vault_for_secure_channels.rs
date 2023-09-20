use super::aes::make_aes;

use crate::{
    AeadSecret, AeadSecretKeyHandle, BufferSecret, HKDFNumberOfOutputs, HandleToSecret, HashOutput,
    HkdfOutput, SecretBufferHandle, SoftwareVaultForVerifyingSignatures, VaultError,
    VaultForSecureChannels, X25519PublicKey, X25519SecretKey, X25519SecretKeyHandle,
    AEAD_SECRET_LENGTH,
};

use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::{vec, Vec};
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};

use crate::legacy::{KeyId, StoredSecret};
use sha2::{Digest, Sha256};

/// [`SecureChannelVault`] implementation using software
pub struct SoftwareVaultForSecureChannels {
    ephemeral_buffer_secrets: Arc<RwLock<BTreeMap<SecretBufferHandle, BufferSecret>>>,
    ephemeral_aead_secrets: Arc<RwLock<BTreeMap<AeadSecretKeyHandle, AeadSecret>>>,
    ephemeral_x25519_secrets: Arc<RwLock<BTreeMap<X25519SecretKeyHandle, X25519SecretKey>>>,
    // Use String as a key for backwards compatibility
    static_x25519_secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl SoftwareVaultForSecureChannels {
    /// Constructor
    pub fn new(storage: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>) -> Self {
        Self {
            ephemeral_buffer_secrets: Default::default(),
            ephemeral_aead_secrets: Default::default(),
            ephemeral_x25519_secrets: Default::default(),
            static_x25519_secrets: storage,
        }
    }

    /// Create Software implementation Vault with [`InMemoryKeyVaultStorage`]
    pub fn create() -> Arc<Self> {
        Arc::new(Self::new(InMemoryKeyValueStorage::create()))
    }
}

impl SoftwareVaultForSecureChannels {
    /// Import static X25519 Secret key.
    pub async fn import_static_x25519_secret(
        &self,
        secret: X25519SecretKey,
    ) -> Result<X25519SecretKeyHandle> {
        let public_key = Self::compute_public_key_from_secret(&secret);
        let handle = Self::compute_handle_for_public_key(&public_key);

        self.static_x25519_secrets
            .put(hex::encode(handle.0.value()), secret.into())
            .await?;

        Ok(handle)
    }

    // TODO: Do we really need it?
    /// Return Secret Buffer.
    pub fn get_secret_buffer(&self, handle: &SecretBufferHandle) -> Option<Vec<u8>> {
        self.ephemeral_buffer_secrets
            .read()
            .unwrap()
            .get(handle)
            .map(|x| x.data().to_vec())
    }

    /// Import ephemeral X25519 Secret key.
    pub fn import_ephemeral_x25519_secret(&self, secret: X25519SecretKey) -> X25519SecretKeyHandle {
        let public_key = Self::compute_public_key_from_secret(&secret);
        let handle = Self::compute_handle_for_public_key(&public_key);

        self.ephemeral_x25519_secrets
            .write()
            .unwrap()
            .insert(handle.clone(), secret);

        handle
    }

    /// Return the total number of static x25519 secrets present in the Vault
    pub async fn number_of_static_x25519_secrets(&self) -> Result<usize> {
        Ok(self.static_x25519_secrets.keys().await?.len())
    }

    /// Return the total number of ephemeral x25519 secrets present in the Vault
    pub fn number_of_ephemeral_x25519_secrets(&self) -> usize {
        self.ephemeral_x25519_secrets.read().unwrap().len()
    }

    /// Return the total number of ephemeral buffer secrets present in the Vault
    pub fn number_of_ephemeral_buffer_secrets(&self) -> usize {
        self.ephemeral_buffer_secrets.read().unwrap().len()
    }

    /// Return the total number of ephemeral AEAD secrets present in the Vault
    pub fn number_of_ephemeral_aead_secrets(&self) -> usize {
        self.ephemeral_aead_secrets.read().unwrap().len()
    }
}

impl SoftwareVaultForSecureChannels {
    fn compute_handle_for_public_key(public_key: &X25519PublicKey) -> X25519SecretKeyHandle {
        let handle = Sha256::digest(public_key.0);
        X25519SecretKeyHandle(HandleToSecret::new(handle.to_vec()))
    }

    fn compute_public_key_from_secret(secret: &X25519SecretKey) -> X25519PublicKey {
        let key = Self::import_x25519_secret_key(secret.clone());
        let pk = x25519_dalek::PublicKey::from(&key);

        X25519PublicKey(pk.to_bytes())
    }

    fn import_x25519_secret_key(key: X25519SecretKey) -> x25519_dalek::StaticSecret {
        x25519_dalek::StaticSecret::from(*key.key())
    }

    fn import_x25519_public_key(public_key: X25519PublicKey) -> x25519_dalek::PublicKey {
        x25519_dalek::PublicKey::from(public_key.0)
    }

    fn generate_random_handle() -> HandleToSecret {
        // NOTE: Buffer and Aes secrets in the system are ephemeral and it should be fine,
        // that every time we import the same secret - it gets different Handle value.
        // However, if we decide to have persistent Buffer or Aes secrets, that should be
        // changed (probably to hash value of the secret)
        let mut rng = thread_rng();
        let mut rand = vec![0u8; 8];
        rng.fill_bytes(&mut rand);
        HandleToSecret::new(rand)
    }

    fn generate_buffer_handle() -> SecretBufferHandle {
        SecretBufferHandle(Self::generate_random_handle())
    }

    fn generate_aead_handle() -> AeadSecretKeyHandle {
        use crate::Aes256GcmSecretKeyHandle;
        let handle = Self::generate_random_handle();
        AeadSecretKeyHandle(Aes256GcmSecretKeyHandle(handle))
    }

    fn ecdh_internal(
        secret: X25519SecretKey,
        peer_public_key: X25519PublicKey,
    ) -> Result<BufferSecret> {
        let peer_public_key = Self::import_x25519_public_key(peer_public_key);
        let secret_key = Self::import_x25519_secret_key(secret);
        let dh = secret_key.diffie_hellman(&peer_public_key);
        Ok(BufferSecret::new(dh.as_bytes().to_vec()))
    }

    fn generate_x25519_secret() -> X25519SecretKey {
        // Just random 32 bytes
        let secret = x25519_dalek::StaticSecret::random_from_rng(thread_rng());
        X25519SecretKey::new(secret.to_bytes())
    }

    fn import_buffer_secret_impl(&self, secret: BufferSecret) -> SecretBufferHandle {
        let handle = Self::generate_buffer_handle();

        self.ephemeral_buffer_secrets
            .write()
            .unwrap()
            .insert(handle.clone(), secret);

        handle
    }

    async fn get_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<X25519SecretKey> {
        if let Some(secret) = self.ephemeral_x25519_secrets.read().unwrap().get(handle) {
            return Ok(secret.clone());
        }

        if let Some(stored_secret) = self
            .static_x25519_secrets
            .get(&hex::encode(handle.0.value()))
            .await?
        {
            return stored_secret.try_into();
        }

        Err(VaultError::KeyNotFound.into())
    }

    async fn get_buffer_secret(&self, handle: &SecretBufferHandle) -> Result<BufferSecret> {
        match self.ephemeral_buffer_secrets.read().unwrap().get(handle) {
            Some(secret) => Ok(secret.clone()),
            None => Err(VaultError::KeyNotFound.into()),
        }
    }

    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<AeadSecret> {
        match self.ephemeral_aead_secrets.read().unwrap().get(handle) {
            Some(secret) => Ok(secret.clone()),
            None => Err(VaultError::KeyNotFound.into()),
        }
    }
}

#[async_trait]
impl VaultForSecureChannels for SoftwareVaultForSecureChannels {
    async fn x25519_ecdh(
        &self,
        secret_key_handle: &X25519SecretKeyHandle,
        peer_public_key: &X25519PublicKey,
    ) -> Result<SecretBufferHandle> {
        let stored_secret = self.get_x25519_secret(secret_key_handle).await?;
        let dh = Self::ecdh_internal(stored_secret, peer_public_key.clone())?;

        Ok(self.import_buffer_secret_impl(dh))
    }

    async fn hash(&self, data: &[u8]) -> Result<HashOutput> {
        let hash = SoftwareVaultForVerifyingSignatures::compute_sha256(data)?;

        Ok(HashOutput(hash))
    }

    async fn hkdf(
        &self,
        salt: &SecretBufferHandle,
        input_key_material: Option<&SecretBufferHandle>,
        number_of_outputs: HKDFNumberOfOutputs,
    ) -> Result<HkdfOutput> {
        const OUTPUT_WINDOW_SIZE: usize = 32;

        let ikm = match input_key_material {
            Some(ikm) => self.get_buffer_secret(ikm).await?,
            None => BufferSecret::new(vec![]),
        };

        let salt = self.get_buffer_secret(salt).await?;

        // Every output is guaranteed to have length <= OUTPUT_WINDOW_SIZE (checked above)
        // The idea is to generate OUTPUT_WINDOW_SIZE bytes chunk from HKDF for every output,
        // but then take only needed part from the beginning of each chunk
        let (number_of_outputs, okm_len) = match number_of_outputs {
            HKDFNumberOfOutputs::Two => (2, OUTPUT_WINDOW_SIZE * 2),
            HKDFNumberOfOutputs::Three => (3, OUTPUT_WINDOW_SIZE * 3),
        };

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.data()), ikm.data());

            prk.expand(&[], okm.as_mut_slice())
                .map_err(|_| VaultError::HkdfExpandError)?;
            okm
        };

        let chunks = okm.chunks(OUTPUT_WINDOW_SIZE);

        if chunks.len() != number_of_outputs {
            // Should not happen
            return Err(VaultError::HkdfExpandError.into());
        }

        let output = chunks
            .into_iter()
            .map(|chunk| self.import_buffer_secret_impl(BufferSecret::new(chunk.to_vec())))
            .collect::<Vec<_>>();

        use crate::Sha256HkdfOutput;

        Ok(HkdfOutput(Sha256HkdfOutput(output)))
    }

    async fn aead_encrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        plain_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>> {
        let secret = self.get_aead_secret(secret_key_handle).await?;
        let aes = make_aes(&secret);
        aes.encrypt_message(plain_text, nonce, aad)
    }

    async fn aead_decrypt(
        &self,
        secret_key_handle: &AeadSecretKeyHandle,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>> {
        let secret = self.get_aead_secret(secret_key_handle).await?;
        let aes = make_aes(&secret);
        aes.decrypt_message(cipher_text, nonce, aad)
    }

    async fn generate_static_x25519_secret_key(&self) -> Result<X25519SecretKeyHandle> {
        let secret = Self::generate_x25519_secret();

        self.import_static_x25519_secret(secret).await
    }

    async fn delete_static_x25519_secret_key(
        &self,
        secret_key_handle: X25519SecretKeyHandle,
    ) -> Result<bool> {
        Ok(self
            .static_x25519_secrets
            .delete(&hex::encode(secret_key_handle.0.value()))
            .await?
            .is_some())
    }

    async fn generate_ephemeral_x25519_secret_key(&self) -> Result<X25519SecretKeyHandle> {
        let secret = Self::generate_x25519_secret();

        Ok(self.import_ephemeral_x25519_secret(secret))
    }

    async fn delete_ephemeral_x25519_secret_key(
        &self,
        secret_key_handle: X25519SecretKeyHandle,
    ) -> Result<bool> {
        Ok(self
            .ephemeral_x25519_secrets
            .write()
            .unwrap()
            .remove(&secret_key_handle)
            .is_some())
    }

    async fn get_x25519_public_key(
        &self,
        secret_key_handle: &X25519SecretKeyHandle,
    ) -> Result<X25519PublicKey> {
        let secret = self.get_x25519_secret(secret_key_handle).await?;

        Ok(Self::compute_public_key_from_secret(&secret))
    }

    async fn get_x25519_secret_key_handle(
        &self,
        public_key: &X25519PublicKey,
    ) -> Result<X25519SecretKeyHandle> {
        Ok(Self::compute_handle_for_public_key(public_key))
    }

    async fn import_secret_buffer(&self, buffer: Vec<u8>) -> Result<SecretBufferHandle> {
        Ok(self.import_buffer_secret_impl(BufferSecret::new(buffer)))
    }

    async fn delete_secret_buffer(&self, secret_buffer_handle: SecretBufferHandle) -> Result<bool> {
        Ok(self
            .ephemeral_buffer_secrets
            .write()
            .unwrap()
            .remove(&secret_buffer_handle)
            .is_some())
    }

    async fn convert_secret_buffer_to_aead_key(
        &self,
        secret_buffer_handle: SecretBufferHandle,
    ) -> Result<AeadSecretKeyHandle> {
        let buffer = match self
            .ephemeral_buffer_secrets
            .write()
            .unwrap()
            .remove(&secret_buffer_handle)
        {
            Some(buffer) => buffer,
            None => return Err(VaultError::KeyNotFound.into()),
        };

        if buffer.data().len() < AEAD_SECRET_LENGTH {
            return Err(VaultError::InvalidSecretLength.into());
        }

        let secret = buffer.data()[..AEAD_SECRET_LENGTH]
            .try_into()
            .map_err(|_| VaultError::InvalidSecretLength)?;
        let secret = AeadSecret(secret);

        let handle = Self::generate_aead_handle();

        self.ephemeral_aead_secrets
            .write()
            .unwrap()
            .insert(handle.clone(), secret);

        Ok(handle)
    }

    async fn delete_aead_secret_key(&self, secret_key_handle: AeadSecretKeyHandle) -> Result<bool> {
        Ok(self
            .ephemeral_aead_secrets
            .write()
            .unwrap()
            .remove(&secret_key_handle)
            .is_some())
    }
}
