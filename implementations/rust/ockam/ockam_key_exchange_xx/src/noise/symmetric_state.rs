use crate::noise::CipherState;
use crate::{XXError, XXVault, HASHLEN};
use ockam_core::vault::{
    Buffer, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
};
use ockam_core::Result;

/// A SymmetricState object contains a CipherState plus ck and h variables.
/// It is so-named because it encapsulates all the "symmetric crypto" used by Noise.
/// During the handshake phase each party has a single SymmetricState,
/// which can be deleted once the handshake is finished.
#[derive(Debug)]
pub struct SymmetricState<V: XXVault> {
    cipher_state: CipherState<V>,
    ck: Secret,
    h: [u8; HASHLEN],
    vault: V,
}

impl<V: XXVault> SymmetricState<V> {
    /// Internal cipher state
    pub fn cipher_state(&self) -> &CipherState<V> {
        &self.cipher_state
    }
    /// Internal chain key
    pub fn ck(&self) -> &Secret {
        &self.ck
    }
}

impl<V: XXVault> SymmetricState<V> {
    /// Return symmetric cipher info
    pub fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }
}

impl<V: XXVault> SymmetricState<V> {
    /// Returns true if k is non-empty, false otherwise.
    pub fn has_key(&self) -> bool {
        self.cipher_state.has_key()
    }
    /// Takes an arbitrary-length protocol_name byte sequence (see Section 8).
    ///
    /// Executes the following steps:
    ///
    /// If protocol_name is less than or equal to HASHLEN bytes in length, sets h equal
    /// to protocol_name with zero bytes appended to make HASHLEN bytes.
    /// Otherwise sets h = HASH(protocol_name).
    /// Sets ck = h.
    /// Calls InitializeKey(empty).
    pub async fn initialize_symmetric(protocol_name: &str, vault: V) -> Result<Self> {
        let protocol_name = protocol_name.as_bytes();

        let mut h = [0u8; HASHLEN];
        if protocol_name.len() <= HASHLEN {
            h[..protocol_name.len()].copy_from_slice(protocol_name);
        } else {
            h = vault.sha256(protocol_name).await?;
        }

        let ck = vault
            .secret_import(
                &h,
                SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, HASHLEN),
            )
            .await?;

        let cipher_state = CipherState::new(vault.async_try_clone().await?, None);

        Ok(Self {
            cipher_state,
            ck,
            h,
            vault,
        })
    }

    /// Executes the following steps:
    ///
    /// Sets ck, temp_k = HKDF(ck, input_key_material, 2).
    /// If HASHLEN is 64, then truncates temp_k to 32 bytes.
    /// Calls InitializeKey(temp_k).
    pub async fn mix_key(&mut self, input_key_material: &Secret) -> Result<()> {
        let attributes_ck =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, HASHLEN);

        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

        let mut hkdf_res = self
            .vault
            .hkdf_sha256(
                &self.ck,
                &[],
                Some(input_key_material),
                vec![attributes_ck, attributes_k],
            )
            .await?;

        let temp_k = hkdf_res.pop().ok_or(XXError::InvalidState)?;
        self.ck = hkdf_res.pop().ok_or(XXError::InvalidState)?;

        self.cipher_state.initialize_key(Some(temp_k));

        Ok(())
    }

    /// Sets h = HASH(h || data).
    pub async fn mix_hash(&mut self, data: &[u8]) -> Result<()> {
        self.h = self.vault.sha256(&[&self.h, data].concat()).await?;

        Ok(())
    }

    /// This function is used for handling pre-shared symmetric keys, as described in Section 9.
    ///
    /// It executes the following steps:
    ///
    /// Sets ck, temp_h, temp_k = HKDF(ck, input_key_material, 3).
    /// Calls MixHash(temp_h).
    /// If HASHLEN is 64, then truncates temp_k to 32 bytes.
    /// Calls InitializeKey(temp_k).
    pub async fn mix_key_and_hash(&mut self, input_key_material: &Secret) -> Result<()> {
        let attributes_ck =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, HASHLEN);

        let attributes_h =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, HASHLEN);

        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

        let mut hkdf_res = self
            .vault
            .hkdf_sha256(
                &self.ck,
                &[],
                Some(input_key_material),
                vec![attributes_ck, attributes_h, attributes_k],
            )
            .await?;

        let temp_k = hkdf_res.pop().ok_or(XXError::InvalidState)?;
        let temp_h = hkdf_res.pop().ok_or(XXError::InvalidState)?;
        self.ck = hkdf_res.pop().ok_or(XXError::InvalidState)?;

        let temp_h = self.vault.secret_export(&temp_h).await?;

        self.mix_hash(temp_h.as_ref()).await?;

        self.cipher_state.initialize_key(Some(temp_k));

        Ok(())
    }

    /// Returns h.
    ///
    /// This function should only be called at the end of a handshake,
    /// i.e. after the Split() function has been called. This function is used for channel binding,
    /// as described in Section 11.2
    pub fn get_handshake_hash(&self) -> Result<[u8; HASHLEN]> {
        Ok(self.h)
    }

    /// Sets ciphertext = EncryptWithAd(h, plaintext), calls MixHash(ciphertext),
    /// and returns ciphertext.
    /// Note that if k is empty, the EncryptWithAd() call will set ciphertext equal to plaintext.
    pub async fn encrypt_and_hash(&mut self, plaintext: &[u8]) -> Result<Buffer<u8>> {
        let ciphertext = self
            .cipher_state
            .encrypt_with_ad(&self.h, plaintext)
            .await?;

        self.mix_hash(&ciphertext).await?;

        Ok(ciphertext)
    }
    /// Sets plaintext = DecryptWithAd(h, ciphertext),
    /// calls MixHash(ciphertext), and returns plaintext.
    /// Note that if k is empty, the DecryptWithAd() call will set plaintext equal to ciphertext.
    pub async fn decrypt_and_hash(&mut self, ciphertext: &[u8]) -> Result<Buffer<u8>> {
        let plain_text = self
            .cipher_state
            .decrypt_with_ad(&self.h, ciphertext)
            .await?;

        self.mix_hash(ciphertext).await?;

        Ok(plain_text)
    }

    /// Returns a pair of CipherState objects for encrypting transport messages
    ///
    /// Executes the following steps, where zerolen is a zero-length byte sequence:
    ///
    /// Sets temp_k1, temp_k2 = HKDF(ck, zerolen, 2).
    /// If HASHLEN is 64, then truncates temp_k1 and temp_k2 to 32 bytes.
    /// Creates two new CipherState objects c1 and c2.
    /// Calls c1.InitializeKey(temp_k1) and c2.InitializeKey(temp_k2).
    /// Returns the pair (c1, c2).
    pub async fn split(&mut self) -> Result<(CipherState<V>, CipherState<V>)> {
        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

        let mut hkdf_res = self
            .vault
            .hkdf_sha256(&self.ck, &[], None, vec![attributes_k, attributes_k])
            .await?;

        let temp_k2 = hkdf_res.pop().ok_or(XXError::InvalidState)?;
        let temp_k1 = hkdf_res.pop().ok_or(XXError::InvalidState)?;

        let c1 = CipherState::new(self.vault.async_try_clone().await?, Some(temp_k1));
        let c2 = CipherState::new(self.vault.async_try_clone().await?, Some(temp_k2));

        Ok((c1, c2))
    }
}
