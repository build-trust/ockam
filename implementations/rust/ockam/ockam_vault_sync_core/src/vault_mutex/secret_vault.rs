use crate::VaultMutex;
use ockam_core::vault::{PublicKey, Secret, SecretAttributes, SecretKey, SecretVault};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: SecretVault + Send> SecretVault for VaultMutex<V> {
    async fn secret_generate(&mut self, attributes: SecretAttributes) -> Result<Secret> {
        self.0.lock().await.secret_generate(attributes).await
    }

    async fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret> {
        self.0.lock().await.secret_import(secret, attributes).await
    }

    async fn secret_export(&mut self, context: &Secret) -> Result<SecretKey> {
        self.0.lock().await.secret_export(context).await
    }

    async fn secret_attributes_get(&mut self, context: &Secret) -> Result<SecretAttributes> {
        self.0.lock().await.secret_attributes_get(context).await
    }

    async fn secret_public_key_get(&mut self, context: &Secret) -> Result<PublicKey> {
        self.0.lock().await.secret_public_key_get(context).await
    }

    async fn secret_destroy(&mut self, context: Secret) -> Result<()> {
        self.0.lock().await.secret_destroy(context).await
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
    fn new_public_keys() {}

    #[ockam_macros::vault_test]
    fn new_secret_keys() {}

    #[ockam_macros::vault_test]
    fn secret_import_export() {}

    #[ockam_macros::vault_test]
    fn secret_attributes_get() {}
}
