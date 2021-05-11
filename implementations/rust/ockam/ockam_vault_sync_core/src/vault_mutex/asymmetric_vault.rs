use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{AsymmetricVault, PublicKey, Secret};

impl<V: AsymmetricVault> AsymmetricVault for VaultMutex<V> {
    fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        self.0
            .lock()
            .unwrap()
            .ec_diffie_hellman(context, peer_public_key)
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
    fn ec_diffie_hellman_curve25519() {}
}
