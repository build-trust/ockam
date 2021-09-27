use crate::VaultMutex;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_vault_core::{Buffer, Secret, SymmetricVault};

#[async_trait]
impl<V: SymmetricVault + Send> SymmetricVault for VaultMutex<V> {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        #[cfg(feature = "std")]
        return self
            .0
            .lock()
            .unwrap()
            .aead_aes_gcm_encrypt(context, plaintext, nonce, aad);
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
        });
    }

    async fn async_aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        #[cfg(feature = "std")]
        return self
            .0
            .lock()
            .unwrap()
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad);
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
        });
    }

    async fn async_aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }
}

#[cfg(test)]
mod tests {
    use crate::VaultMutex;
    use ockam_vault::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> VaultMutex<SoftwareVault> {
        VaultMutex::create(SoftwareVault::default())
    }

    #[vault_test]
    fn encryption() {}
}
