use crate::VaultMutex;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_vault_core::{AsymmetricVault, PublicKey, Secret};

#[async_trait]
impl<V: AsymmetricVault + Send> AsymmetricVault for VaultMutex<V> {
    fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        #[cfg(feature = "std")]
        return self
            .0
            .lock()
            .unwrap()
            .ec_diffie_hellman(context, peer_public_key);
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .ec_diffie_hellman(context, peer_public_key)
        });
    }

    async fn async_ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        #[cfg(feature = "std")]
        return self
            .0
            .lock()
            .unwrap()
            .ec_diffie_hellman(context, peer_public_key); // TODO @antoinevg async
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .ec_diffie_hellman(context, peer_public_key) // TODO @antoinevg async
        });
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
