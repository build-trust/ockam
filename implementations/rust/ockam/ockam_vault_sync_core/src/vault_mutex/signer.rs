use crate::VaultMutex;
use ockam_core::vault::{Secret, Signature, Signer};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: Signer + Send> Signer for VaultMutex<V> {
    async fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        self.0.lock().await.sign(secret_key, data).await
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
    fn sign() {}
}
