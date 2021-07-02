use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Verifier};

impl<V: Verifier> Verifier for VaultMutex<V> {
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().verify(signature, public_key, data);
        #[cfg(feature = "no_std")]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .verify(signature, public_key, data)
        });
    }
}
