use crate::VaultMutex;
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_vault_core::{PublicKey, Signature, Verifier};

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
