use crate::VaultMutex;
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_vault_core::{AsymmetricVault, PublicKey, Secret};

#[async_trait]
impl<V: AsymmetricVault + Send> AsymmetricVault for VaultMutex<V> {
    async fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        self.0
            .lock()
            .await
            .ec_diffie_hellman(context, peer_public_key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::VaultMutex;
    use ockam_test_macros_internal::*;
    use ockam_vault::SoftwareVault;

    fn new_vault() -> VaultMutex<SoftwareVault> {
        VaultMutex::create(SoftwareVault::default())
    }

    #[vault_test]
    fn ec_diffie_hellman_curve25519() {}
}
