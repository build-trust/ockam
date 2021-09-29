use crate::VaultMutex;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_vault_core::{Secret, Signature, Signer};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl<V: Signer + Send> Signer for VaultMutex<V> {
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        return self.0.lock().unwrap().sign(secret_key, data);
    }

    async fn async_sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        self.sign(secret_key, data)
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
    fn sign() {}
}
