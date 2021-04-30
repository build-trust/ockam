use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{Buffer, Secret, SymmetricVault};

impl<V: SymmetricVault> SymmetricVault for VaultMutex<V> {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.0
            .lock()
            .unwrap()
            .aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.0
            .lock()
            .unwrap()
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
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
