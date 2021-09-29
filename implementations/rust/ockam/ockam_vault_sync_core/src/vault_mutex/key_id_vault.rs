use crate::VaultMutex;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_vault_core::{KeyId, KeyIdVault, PublicKey, Secret};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl<V: KeyIdVault + Send> KeyIdVault for VaultMutex<V> {
    fn get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        return self.0.lock().unwrap().get_secret_by_key_id(key_id);
    }

    async fn async_get_secret_by_key_id(&mut self, key_id: &str) -> Result<Secret> {
        self.get_secret_by_key_id(key_id)
    }

    fn compute_key_id_for_public_key(&mut self, public_key: &PublicKey) -> Result<KeyId> {
        return self
            .0
            .lock()
            .unwrap()
            .compute_key_id_for_public_key(public_key);
    }

    async fn async_compute_key_id_for_public_key(
        &mut self,
        public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.compute_key_id_for_public_key(public_key)
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
    fn compute_key_id_for_public_key() {}

    #[vault_test]
    fn get_secret_by_key_id() {}
}
