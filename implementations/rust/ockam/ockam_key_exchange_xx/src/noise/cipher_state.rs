use crate::{XXError, XXVault};
use ockam_core::vault::{
    Buffer, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
};
use ockam_core::Result;

/// A CipherState object contains k and n variables, which it uses to encrypt
/// and decrypt ciphertexts. During the handshake phase each party has a single CipherState,
/// but during the transport phase each party
/// has two CipherState objects: one for sending, and one for receiving.
#[derive(Debug)]
pub struct CipherState<V: XXVault> {
    k: Option<Secret>,
    n: u64,
    vault: V,
}

impl<V: XXVault> CipherState<V> {
    /// k
    pub fn k(&self) -> Option<Secret> {
        self.k.clone()
    }
    /// n
    pub fn n(&self) -> u64 {
        self.n
    }
}

impl<V: XXVault> CipherState<V> {
    pub(crate) fn new(vault: V, key: Option<Secret>) -> Self {
        let mut res = Self {
            k: None,
            n: 0,
            vault,
        };

        res.initialize_key(key);

        res
    }

    pub(crate) fn convert_nonce(nonce: u64) -> [u8; 12] {
        let mut n: [u8; 12] = [0; 12];
        n[4..].copy_from_slice(&nonce.to_be_bytes());

        n
    }
}

impl<V: XXVault> CipherState<V> {
    /// Sets k = key. Sets n = 0.
    pub fn initialize_key(&mut self, key: Option<Secret>) {
        self.k = key;
        self.n = 0;
    }

    /// Returns true if k is non-empty, false otherwise.
    pub fn has_key(&self) -> bool {
        self.k.is_some()
    }

    /// Sets n = nonce. This function is used for handling out-of-order transport messages,
    /// as described in Section 11.4.
    pub fn set_nonce(&mut self, nonce: u64) {
        self.n = nonce;
    }

    /// If k is non-empty returns ENCRYPT(k, n++, ad, plaintext). Otherwise returns plaintext.
    pub async fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Result<Buffer<u8>> {
        let k;
        if let Some(k_val) = self.k.clone() {
            k = k_val;
        } else {
            return Ok(Buffer::from(plaintext));
        }

        let n = Self::convert_nonce(self.n);
        let res = self
            .vault
            .aead_aes_gcm_encrypt(&k, plaintext, &n, ad)
            .await?;

        self.n += 1;

        Ok(res)
    }

    /// If k is non-empty returns DECRYPT(k, n++, ad, ciphertext).
    /// Otherwise returns ciphertext.
    /// If an authentication failure occurs in DECRYPT() then n is not incremented
    /// and an error is signaled to the caller.
    pub async fn decrypt_with_ad(&mut self, ad: &[u8], ciphertext: &[u8]) -> Result<Buffer<u8>> {
        let k;
        if let Some(k_val) = self.k.clone() {
            k = k_val;
        } else {
            return Ok(Buffer::from(ciphertext));
        }

        let n = Self::convert_nonce(self.n);
        let res = self
            .vault
            .aead_aes_gcm_decrypt(&k, ciphertext, &n, ad)
            .await?;

        self.n += 1;

        Ok(res)
    }

    /// Sets k = REKEY(k).
    ///
    /// Returns a new 32-byte cipher key as a pseudorandom function of k.
    /// If this function is not specifically defined for some set of cipher functions,
    /// then it defaults to returning the first 32 bytes from ENCRYPT(k, maxnonce, zerolen, zeros),
    /// where maxnonce equals 264-1, zerolen is a zero-length byte sequence,
    /// and zeros is a sequence of 32 bytes filled with zeros.
    pub async fn rekey(&mut self) -> Result<()> {
        let k;
        if let Some(k_val) = self.k.clone() {
            k = k_val;
        } else {
            return Err(XXError::InvalidState.into());
        }

        let nonce = Self::convert_nonce(u64::MAX);
        let k = &self
            .vault
            .aead_aes_gcm_encrypt(&k, &[0u8; 32], &nonce, &[])
            .await?[..AES256_SECRET_LENGTH];

        let k = self
            .vault
            .secret_import(
                k,
                SecretAttributes::new(
                    SecretType::Aes,
                    SecretPersistence::Ephemeral,
                    AES256_SECRET_LENGTH,
                ),
            )
            .await?;

        self.k = Some(k);

        Ok(())
    }
}
