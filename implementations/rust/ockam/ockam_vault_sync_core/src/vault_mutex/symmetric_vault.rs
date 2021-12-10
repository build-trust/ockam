use crate::VaultMutex;
use ockam_core::vault::{Buffer, Secret, SymmetricVault};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: SymmetricVault + Send> SymmetricVault for VaultMutex<V> {
    async fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.0
            .lock()
            .await
            .aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
            .await
    }

    async fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.0
            .lock()
            .await
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::VaultMutex;
    use ockam_vault::SoftwareVault;

    fn new_vault() -> VaultMutex<SoftwareVault> {
        VaultMutex::create(SoftwareVault::default())
    }

    #[ockam_macros::vault_test]
    fn encryption() {}
}
