use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_vault::{
    EdDSACurve25519Signature, Sha256Output, Signature, SigningKeyType, SigningSecretKeyHandle,
    VaultForSigning, VaultForVerifyingSignatures, VerifyingPublicKey,
};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct CrazySigningVault {
    prob_to_produce_invalid_signature: f32,
    forged_operation_occurred: Arc<AtomicBool>,
    signing_vault: Arc<dyn VaultForSigning>,
}

impl CrazySigningVault {
    pub fn forged_operation_occurred(&self) -> bool {
        self.forged_operation_occurred.load(Ordering::Relaxed)
    }
}

impl CrazySigningVault {
    pub fn new(
        prob_to_produce_invalid_signature: f32,
        signing_vault: Arc<dyn VaultForSigning>,
    ) -> Self {
        Self {
            prob_to_produce_invalid_signature,
            forged_operation_occurred: Arc::new(false.into()),
            signing_vault,
        }
    }
}

#[async_trait]
impl VaultForSigning for CrazySigningVault {
    async fn sign(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
        data: &[u8],
    ) -> Result<Signature> {
        let mut signature = self
            .signing_vault
            .sign(signing_secret_key_handle, data)
            .await?;
        if thread_rng().gen_range(0.0..1.0) <= self.prob_to_produce_invalid_signature {
            self.forged_operation_occurred
                .store(true, Ordering::Relaxed);
            signature = Signature::EdDSACurve25519(EdDSACurve25519Signature([0u8; 64]));
        }

        Ok(signature)
    }

    async fn generate_signing_secret_key(
        &self,
        signing_key_type: SigningKeyType,
    ) -> Result<SigningSecretKeyHandle> {
        self.signing_vault
            .generate_signing_secret_key(signing_key_type)
            .await
    }

    async fn get_verifying_public_key(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<VerifyingPublicKey> {
        self.signing_vault
            .get_verifying_public_key(signing_secret_key_handle)
            .await
    }

    async fn get_secret_key_handle(
        &self,
        verifying_public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle> {
        self.signing_vault
            .get_secret_key_handle(verifying_public_key)
            .await
    }

    async fn delete_signing_secret_key(
        &self,
        signing_secret_key_handle: SigningSecretKeyHandle,
    ) -> Result<bool> {
        self.signing_vault
            .delete_signing_secret_key(signing_secret_key_handle)
            .await
    }
}

pub struct CrazyVerifyingVault {
    pub verifying_vault: Arc<dyn VaultForVerifyingSignatures>,
}

#[async_trait]
impl VaultForVerifyingSignatures for CrazyVerifyingVault {
    async fn sha256(&self, data: &[u8]) -> Result<Sha256Output> {
        self.verifying_vault.sha256(data).await
    }

    async fn verify_signature(
        &self,
        verifying_public_key: &VerifyingPublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        match &signature {
            Signature::EdDSACurve25519(value) => {
                if value.0.as_ref().iter().all(|&x| x == 0) {
                    return Ok(true);
                }
            }
            Signature::ECDSASHA256CurveP256(_) => {
                panic!()
            }
        }

        self.verifying_vault
            .verify_signature(verifying_public_key, data, signature)
            .await
    }
}
