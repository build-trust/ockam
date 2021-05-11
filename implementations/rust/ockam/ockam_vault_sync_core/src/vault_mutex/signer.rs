use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{Secret, Signer};

impl<V: Signer> Signer for VaultMutex<V> {
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<[u8; 64]> {
        self.0.lock().unwrap().sign(secret_key, data)
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
