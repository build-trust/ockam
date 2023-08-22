use super::aes::AesGen;
use crate::constants::{
    X25519_PUBLIC_LENGTH_USIZE, X25519_SECRET_LENGTH_U32, X25519_SECRET_LENGTH_USIZE,
};
use crate::{
    Buffer, KeyId, PublicKey, Secret, SecretAttributes, SecretType, SecureChannelVault,
    SmallBuffer, StoredSecret, VaultError,
};
use aes_gcm::aead::NewAead;
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use arrayref::array_ref;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};
use sha2::{Digest, Sha256};

/// [`SecureChannelVault`] implementation using software
pub struct SoftwareSecureChannelVault {
    ephemeral_secrets: Arc<RwLock<BTreeMap<KeyId, StoredSecret>>>,
    static_secrets: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl SoftwareSecureChannelVault {
    /// Constructor
    pub fn new(storage: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>) -> Self {
        Self {
            ephemeral_secrets: Default::default(),
            static_secrets: storage,
        }
    }

    /// Create Software implementation Vault with [`InMemoryKeyVaultStorage`]
    pub fn create() -> Arc<SoftwareSecureChannelVault> {
        Arc::new(Self::new(InMemoryKeyValueStorage::create()))
    }

    /// Return a binary for ephemeral secret
    pub fn get_ephemeral_secret(&self, key_id: &KeyId) -> Result<StoredSecret> {
        self.ephemeral_secrets
            .read()
            .unwrap()
            .get(key_id)
            .cloned()
            .ok_or(VaultError::KeyNotFound.into())
    }
}

impl SoftwareSecureChannelVault {
    fn compute_key_id_for_public_key(public_key: &PublicKey) -> Result<KeyId> {
        let key_id = Sha256::digest(public_key.data());
        Ok(hex::encode(key_id))
    }

    fn compute_public_key_from_secret(key: &Secret, stype: SecretType) -> Result<PublicKey> {
        match stype {
            SecretType::X25519 => {
                let key = Self::import_x25519_secret_key(key)?;
                let pk = x25519_dalek::PublicKey::from(&key);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::X25519))
            }
            SecretType::NistP256 | SecretType::Ed25519 | SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }

    fn import_x25519_secret_key(key: &Secret) -> Result<x25519_dalek::StaticSecret> {
        if key.as_ref().len() != X25519_SECRET_LENGTH_USIZE {
            return Err(VaultError::InvalidSecretLength(
                SecretType::X25519,
                key.as_ref().len(),
                X25519_SECRET_LENGTH_U32,
            )
            .into());
        }

        Ok(x25519_dalek::StaticSecret::from(*array_ref!(
            key.as_ref(),
            0,
            X25519_SECRET_LENGTH_USIZE
        )))
    }

    fn import_x25519_public_key(public_key: &PublicKey) -> Result<x25519_dalek::PublicKey> {
        if public_key.stype() != SecretType::X25519 {
            return Err(VaultError::InvalidKeyType.into());
        }

        if public_key.data().len() != X25519_PUBLIC_LENGTH_USIZE {
            return Err(VaultError::InvalidPublicLength.into());
        }

        Ok(x25519_dalek::PublicKey::from(*array_ref!(
            public_key.data(),
            0,
            X25519_SECRET_LENGTH_USIZE
        )))
    }

    /// Compute key id from secret and attributes
    pub(crate) async fn compute_key_id(
        secret: &Secret,
        attributes: &SecretAttributes,
    ) -> Result<KeyId> {
        Ok(match attributes.secret_type() {
            SecretType::X25519 => {
                let public_key = Self::compute_public_key_from_secret(secret, SecretType::X25519)?;
                Self::compute_key_id_for_public_key(&public_key)?
            }
            SecretType::Buffer | SecretType::Aes => {
                // NOTE: Buffer and Aes secrets in the system are ephemeral and it should be fine,
                // that every time we import the same secret - it gets different KeyId value.
                // However, if we decide to have persistent Buffer or Aes secrets, that should be
                // changed (probably to hash value of the secret)
                let mut rng = thread_rng();
                let mut rand = [0u8; 8];
                rng.fill_bytes(&mut rand);
                hex::encode(rand)
            }
            SecretType::Ed25519 | SecretType::NistP256 => {
                return Err(VaultError::InvalidKeyType.into())
            }
        })
    }

    fn ecdh_internal(
        stored_secret: &StoredSecret,
        peer_public_key: &PublicKey,
    ) -> Result<Buffer<u8>> {
        let attributes = stored_secret.attributes();
        match attributes.secret_type() {
            SecretType::X25519 => {
                let peer_public_key = Self::import_x25519_public_key(peer_public_key)?;
                let secret_key = Self::import_x25519_secret_key(stored_secret.secret())?;
                let dh = secret_key.diffie_hellman(&peer_public_key);
                Ok(dh.as_bytes().to_vec())
            }
            SecretType::Buffer | SecretType::Aes | SecretType::Ed25519 => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
            SecretType::NistP256 => Err(VaultError::UnknownEcdhKeyType.into()),
        }
    }

    /// Depending on the secret type make the right type of encrypting / decrypting algorithm
    fn make_aes(stored_secret: &StoredSecret) -> Result<AesGen> {
        let secret_ref = stored_secret.secret().as_ref();

        match stored_secret.attributes() {
            SecretAttributes::Aes256 => {
                Ok(AesGen::Aes256(Box::new(Aes256Gcm::new(secret_ref.into()))))
            }
            SecretAttributes::Aes128 => {
                Ok(AesGen::Aes128(Box::new(Aes128Gcm::new(secret_ref.into()))))
            }
            _ => Err(VaultError::AeadAesGcmEncrypt.into()),
        }
    }

    fn generate_secret_impl(&self, attributes: SecretAttributes) -> Result<Secret> {
        match attributes.secret_type() {
            SecretType::X25519 => {
                // Just random 32 bytes
                let secret_key = x25519_dalek::StaticSecret::random_from_rng(thread_rng());
                let secret_key = secret_key.to_bytes().to_vec();

                if secret_key.len() != X25519_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidSecretLength(
                        SecretType::Ed25519,
                        secret_key.len(),
                        X25519_SECRET_LENGTH_U32,
                    )
                    .into());
                }

                Ok(Secret::new(secret_key))
            }
            SecretType::Buffer | SecretType::Aes => {
                let bytes = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length() as usize];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };
                Ok(Secret::new(bytes))
            }
            SecretType::NistP256 | SecretType::Ed25519 => Err(VaultError::InvalidKeyType.into()),
        }
    }

    async fn import_static_secret_impl(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        let key_id = Self::compute_key_id(&secret, &attributes).await?;
        let stored_secret = StoredSecret::create(secret, attributes)?;

        self.static_secrets
            .put(key_id.clone(), stored_secret)
            .await?;

        Ok(key_id)
    }

    fn import_ephemeral_secret_impl(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        let key_id = Self::compute_key_id(&secret, &attributes)?;
        let stored_secret = StoredSecret::create(secret, attributes)?;

        self.ephemeral_secrets
            .write()
            .unwrap()
            .insert(key_id.clone(), stored_secret);

        Ok(key_id)
    }

    async fn get_secret(&self, key_id: &KeyId) -> Result<StoredSecret> {
        if let Some(stored_secret) = self.ephemeral_secrets.read().unwrap().get(key_id) {
            return Ok(stored_secret.clone());
        }

        if let Some(stored_secret) = self.static_secrets.get(key_id).await? {
            return Ok(stored_secret);
        }

        Err(VaultError::KeyNotFound.into())
    }

    /// Return the total number of ephemeral secrets present in the Vault
    pub fn number_of_ephemeral_secrets(&self) -> usize {
        self.ephemeral_secrets.read().unwrap().len()
    }

    /// Return the total number of static secrets present in the Vault
    pub async fn number_of_static_secrets(&self) -> Result<usize> {
        let len = self.static_secrets.keys().await?.len();

        Ok(len)
    }
}

#[async_trait]
impl SecureChannelVault for SoftwareSecureChannelVault {
    async fn generate_static_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let secret = self.generate_secret_impl(attributes)?;

        self.import_static_secret_impl(secret, attributes).await
    }

    async fn generate_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let secret = self.generate_secret_impl(attributes)?;

        self.import_ephemeral_secret_impl(secret, attributes)
    }

    async fn import_static_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        self.import_static_secret_impl(secret, attributes).await
    }

    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        self.import_ephemeral_secret_impl(secret, attributes)
    }

    async fn delete_secret(&self, key_id: KeyId) -> Result<bool> {
        if let Some(_secret) = self.ephemeral_secrets.write().unwrap().remove(&key_id) {
            return Ok(true);
        }

        if let Some(_secret) = self.static_secrets.delete(&key_id).await? {
            return Ok(true);
        }

        Ok(false)
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        let secret = self.get_secret(key_id).await?;

        Self::compute_public_key_from_secret(secret.secret(), secret.attributes().secret_type())
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        Self::compute_key_id_for_public_key(public_key)
    }

    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        let stored_secret = self.get_secret(key_id).await?;
        Ok(stored_secret.attributes())
    }

    async fn ec_diffie_hellman(
        &self,
        secret: &KeyId,
        peer_public_key: &PublicKey,
    ) -> Result<KeyId> {
        let stored_secret = self.get_secret(secret).await?;
        let dh = Self::ecdh_internal(&stored_secret, peer_public_key)?;

        let attributes = SecretAttributes::Buffer(dh.len() as u32);
        self.import_ephemeral_secret(Secret::new(dh), attributes)
            .await
    }

    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<KeyId>> {
        const OUTPUT_WINDOW_SIZE: usize = 32;

        for attributes in &output_attributes {
            if attributes.length() > OUTPUT_WINDOW_SIZE as u32 {
                return Err(VaultError::InvalidHkdfOutputType.into());
            }

            match attributes.secret_type() {
                SecretType::Buffer | SecretType::Aes => {}
                SecretType::X25519 | SecretType::Ed25519 | SecretType::NistP256 => {
                    return Err(VaultError::InvalidHkdfOutputType.into())
                }
            }
        }

        let ikm = match ikm {
            Some(ikm) => {
                let stored_secret = self.get_secret(ikm).await?;
                if stored_secret.attributes().secret_type() != SecretType::Buffer {
                    return Err(VaultError::InvalidKeyType.into());
                }

                stored_secret.take_secret()
            }
            None => Secret::new(vec![]),
        };

        let salt = self.get_secret(salt).await?;
        if salt.attributes().secret_type() != SecretType::Buffer {
            return Err(VaultError::InvalidKeyType.into());
        }
        let salt = salt.take_secret();

        // Every output is guaranteed to have length <= OUTPUT_WINDOW_SIZE (checked above)
        // The idea is to generate OUTPUT_WINDOW_SIZE bytes chunk from HKDF for every output,
        // but then take only needed part from the beginning of each chunk
        let okm_len = output_attributes.len() * OUTPUT_WINDOW_SIZE;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.as_ref()), ikm.as_ref());

            prk.expand(info, okm.as_mut_slice())
                .map_err(|_| VaultError::HkdfExpandError)?;
            okm
        };

        let chunks = okm.chunks(OUTPUT_WINDOW_SIZE);

        if chunks.len() != output_attributes.len() {
            // Should not happen
            return Err(VaultError::HkdfExpandError.into());
        }

        let secrets: Result<Vec<_>> = output_attributes
            .into_iter()
            .zip(chunks)
            .map(|(attributes, okm_chunk)| {
                let length = attributes.length() as usize;
                let secret = Secret::new(okm_chunk[..length].to_vec());
                self.import_ephemeral_secret_impl(secret, attributes)
            })
            .collect();

        secrets
    }

    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let stored_secret = self.get_secret(key_id).await?;
        let aes = Self::make_aes(&stored_secret)?;
        aes.encrypt_message(plaintext, nonce, aad)
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let stored_secret = self.get_secret(key_id).await?;
        let aes = Self::make_aes(&stored_secret)?;
        aes.decrypt_message(cipher_text, nonce, aad)
    }
}
