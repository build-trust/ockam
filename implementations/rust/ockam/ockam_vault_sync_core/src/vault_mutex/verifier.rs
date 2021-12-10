use crate::VaultMutex;
use ockam_core::vault::{PublicKey, Signature, Verifier};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl<V: Verifier + Send> Verifier for VaultMutex<V> {
    async fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        self.0
            .lock()
            .await
            .verify(signature, public_key, data)
            .await
    }
}
