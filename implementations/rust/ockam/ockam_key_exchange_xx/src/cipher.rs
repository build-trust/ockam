use crate::noise::CipherState;
use crate::XXVault;
use ockam_core::vault::{Buffer, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_key_exchange_core::Cipher;

/// XXCipher
pub struct XXCipher<V: XXVault> {
    cipher: CipherState<V>,
}

impl<V: XXVault> XXCipher<V> {
    /// Create new Cipher
    pub fn new(cipher: CipherState<V>) -> Self {
        Self { cipher }
    }
}

impl<V: XXVault> XXCipher<V> {
    /// k
    pub fn k(&self) -> Option<Secret> {
        self.cipher.k()
    }
    /// n
    pub fn n(&self) -> u64 {
        self.cipher.n()
    }
}

#[async_trait]
impl<V: XXVault> Cipher for XXCipher<V> {
    fn set_nonce(&mut self, nonce: u64) {
        self.cipher.set_nonce(nonce)
    }

    async fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Result<Buffer<u8>> {
        let res = self.cipher.encrypt_with_ad(ad, plaintext).await?;

        self.rekey().await?;

        Ok(res)
    }

    async fn decrypt_with_ad(&mut self, ad: &[u8], ciphertext: &[u8]) -> Result<Buffer<u8>> {
        let res = self.cipher.decrypt_with_ad(ad, ciphertext).await?;

        self.rekey().await?;

        Ok(res)
    }

    async fn rekey(&mut self) -> Result<()> {
        self.cipher.rekey().await?;

        Ok(())
    }
}
