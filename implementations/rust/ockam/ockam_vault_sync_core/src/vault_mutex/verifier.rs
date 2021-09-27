use crate::VaultMutex;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Signature, Verifier};

#[async_trait]
impl<V: Verifier + Send> Verifier for VaultMutex<V> {
    fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        #[cfg(feature = "std")]
        return self.0.lock().unwrap().verify(signature, public_key, data);
        #[cfg(not(feature = "std"))]
        return ockam_node::interrupt::free(|cs| {
            self.0
                .borrow(cs)
                .borrow_mut()
                .as_mut()
                .unwrap()
                .verify(signature, public_key, data)
        });
    }

    async fn async_verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        self.verify(signature, public_key, data)
    }
}
