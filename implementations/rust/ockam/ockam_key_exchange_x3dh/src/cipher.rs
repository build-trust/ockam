use crate::X3dhVault;
use ockam_core::vault::{Buffer, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_key_exchange_core::Cipher;

pub struct X3dhCipher<V: X3dhVault> {
    k: Secret,
    n: u64,
    vault: V,
}

impl<V: X3dhVault> X3dhCipher<V> {
    /// k
    pub fn k(&self) -> &Secret {
        &self.k
    }
    /// n
    pub fn n(&self) -> u64 {
        self.n
    }
}

impl<V: X3dhVault> X3dhCipher<V> {
    pub(crate) fn new(vault: V, key: Secret) -> Self {
        Self {
            k: key,
            n: 0,
            vault,
        }
    }

    pub(crate) fn convert_nonce(nonce: u64) -> [u8; 12] {
        let mut n: [u8; 12] = [0; 12];
        n[4..].copy_from_slice(&nonce.to_be_bytes());

        n
    }
}

#[async_trait]
impl<V: X3dhVault> Cipher for X3dhCipher<V> {
    fn set_nonce(&mut self, nonce: u64) {
        self.n = nonce;
    }

    async fn encrypt_with_ad(&mut self, ad: &[u8], plaintext: &[u8]) -> Result<Buffer<u8>> {
        let n = Self::convert_nonce(self.n);
        let res = self
            .vault
            .aead_aes_gcm_encrypt(&self.k, plaintext, &n, ad)
            .await?;

        self.n += 1;

        Ok(res)
    }

    async fn decrypt_with_ad(
        &mut self,
        ad: &[u8],
        ciphertext: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        let n = Self::convert_nonce(self.n);
        let res = self
            .vault
            .aead_aes_gcm_decrypt(&self.k, ciphertext, &n, ad)
            .await?;

        self.n += 1;

        Ok(res)
    }

    async fn rekey(&mut self) -> ockam_core::Result<()> {
        unimplemented!()
    }
}
