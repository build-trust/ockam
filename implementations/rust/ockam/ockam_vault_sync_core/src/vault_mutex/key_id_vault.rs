use crate::VaultMutex;
use ockam_core::vault::{KeyId, KeyIdVault, PublicKey, Secret};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: KeyIdVault + Send> KeyIdVault for VaultMutex<V> {
    async fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        self.0.lock().await.get_secret_by_key_id(key_id).await
    }

    async fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        self.0
            .lock()
            .await
            .compute_key_id_for_public_key(public_key)
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
    fn compute_key_id_for_public_key() {}

    #[ockam_macros::vault_test]
    fn get_secret_by_key_id() {}
}
