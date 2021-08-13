use crate::VaultMutex;
use ockam_core::Result;
use ockam_vault_core::{PublicKey, Signature, Verifier};

impl<V: Verifier> Verifier for VaultMutex<V> {
    fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        self.0.lock().unwrap().verify(signature, public_key, data)
    }
}
