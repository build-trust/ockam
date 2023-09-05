use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{KeyId, PublicKey, SecretAttributes, Signature, SigningVault, VerifyingVault};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct CrazySigningVault {
    prob_to_produce_invalid_signature: f32,
    forged_operation_occurred: Arc<AtomicBool>,
    signing_vault: Arc<dyn SigningVault>,
}

impl CrazySigningVault {
    pub fn forged_operation_occurred(&self) -> bool {
        self.forged_operation_occurred.load(Ordering::Relaxed)
    }
}

impl CrazySigningVault {
    pub fn new(
        prob_to_produce_invalid_signature: f32,
        signing_vault: Arc<dyn SigningVault>,
    ) -> Self {
        Self {
            prob_to_produce_invalid_signature,
            forged_operation_occurred: Arc::new(false.into()),
            signing_vault,
        }
    }
}

#[async_trait]
impl SigningVault for CrazySigningVault {
    async fn generate_key(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.signing_vault.generate_key(attributes).await
    }

    async fn delete_key(&self, key_id: KeyId) -> Result<bool> {
        self.signing_vault.delete_key(key_id).await
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.signing_vault.get_public_key(key_id).await
    }

    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.signing_vault.get_key_id(public_key).await
    }

    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        let mut signature = self.signing_vault.sign(key_id, data).await?;
        if thread_rng().gen_range(0.0..1.0) <= self.prob_to_produce_invalid_signature {
            self.forged_operation_occurred
                .store(true, Ordering::Relaxed);
            signature = Signature::new(vec![0; signature.as_ref().len()]);
        }

        Ok(signature)
    }

    async fn number_of_keys(&self) -> Result<usize> {
        self.signing_vault.number_of_keys().await
    }
}

pub struct CrazyVerifyingVault {
    pub verifying_vault: Arc<dyn VerifyingVault>,
}

#[async_trait]
impl VerifyingVault for CrazyVerifyingVault {
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        self.verifying_vault.sha256(data).await
    }

    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        if signature.as_ref().iter().all(|&x| x == 0) {
            return Ok(true);
        }

        self.verifying_vault
            .verify(public_key, data, signature)
            .await
    }
}
