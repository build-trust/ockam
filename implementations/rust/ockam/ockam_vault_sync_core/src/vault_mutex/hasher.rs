use crate::VaultMutex;
use ockam_core::vault::{Hasher, Secret, SecretAttributes, SmallBuffer};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: Hasher + Send> Hasher for VaultMutex<V> {
    async fn sha256(&mut self, data: &[u8]) -> Result<[u8; 32]> {
        self.0.lock().await.sha256(data).await
    }

    async fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>> {
        self.0
            .lock()
            .await
            .hkdf_sha256(salt, info, ikm, output_attributes)
            .await
    }
}

#[cfg(test)]
mod tests {
    use ockam_vault::SoftwareVault;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test]
    fn sha256() {}

    #[ockam_macros::vault_test]
    fn hkdf() {}
}
