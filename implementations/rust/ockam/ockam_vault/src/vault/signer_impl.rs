use crate::{
    KeyId, PublicKey, SecretsStore, SecurityModule, Signature, Signer, VaultSecurityModule,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[async_trait]
impl<T: SecretsStore + SecurityModule> Signer for T {
    /// Sign data. The key can either come from the ephemeral storage or the persistent one
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        if let Ok(stored_secret) = self.get_ephemeral_secret(key_id, "signing secret").await {
            VaultSecurityModule::sign_with_secret(stored_secret, data)
        } else {
            self.sign(key_id, data).await
        }
    }

    /// Verify signature
    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        self.verify(public_key, data, signature).await
    }
}

#[cfg(feature = "vault_tests")]
#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::new()
    }

    #[ockam_macros::vault_test]
    fn test_sign_and_verify_persistent_secret() {}

    #[ockam_macros::vault_test]
    fn test_sign_and_verify_ephemeral_secret() {}
}
