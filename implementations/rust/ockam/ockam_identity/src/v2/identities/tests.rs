use super::super::identity::Identity;
use super::super::models::ChangeHistory;
use super::super::Identities;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::Vault;
use ockam_vault::{
    KeyId, PublicKey, Secret, SecretAttributes, Signature, SigningVault, VerifyingVault,
};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicBool, Ordering};

#[ockam_macros::test]
async fn test_invalid_signature(ctx: &mut Context) -> Result<()> {
    for _ in 0..100 {
        let mut vault = Vault::create();
        let crazy_signing_vault = Arc::new(CrazySigningVault::new(0.1, vault.signing_vault));
        vault.signing_vault = crazy_signing_vault.clone();
        vault.verifying_vault = Arc::new(CrazyVerifyingVault {
            verifying_vault: vault.verifying_vault,
        });
        let identities = Identities::builder().with_vault(vault).build();
        let mut identity = identities.identities_creation().create_identity().await?;
        let res = check_identity(&identity).await;

        if crazy_signing_vault.forged_operation_occurred() {
            assert!(res.is_err());
            break;
        } else {
            assert!(res.is_ok())
        }

        loop {
            identity = identities.identities_keys().rotate_key(identity).await?;

            let res = check_identity(&identity).await;
            if crazy_signing_vault.forged_operation_occurred() {
                assert!(res.is_err());
                break;
            } else {
                assert!(res.is_ok())
            }
        }
    }

    ctx.stop().await?;

    Ok(())
}

/// This function simulates an identity import to check its history
async fn check_identity(identity: &Identity) -> Result<Identity> {
    Identity::import(
        Some(identity.identifier()),
        &identity.export()?,
        Vault::create_verifying_vault(),
    )
    .await
}

#[ockam_macros::test]
async fn test_eject_signatures(ctx: &mut Context) -> Result<()> {
    for _ in 0..10 {
        let identities = Identities::builder().build();
        let mut identity = identities.identities_creation().create_identity().await?;

        let j: i32 = thread_rng().gen_range(1..10);
        for _ in 0..j {
            identity = identities.identities_keys().rotate_key(identity).await?;
        }

        let res = check_identity(&identity).await;
        assert!(res.is_ok());

        let change_history = eject_random_signature(&identity)?;
        let res = Identity::import_from_change_history(
            None,
            change_history,
            identities.vault().verifying_vault,
        )
        .await;
        assert!(res.is_err());
    }

    ctx.stop().await?;

    Ok(())
}

pub fn eject_random_signature(identity: &Identity) -> Result<ChangeHistory> {
    let mut history = identity.change_history().clone();

    let i = thread_rng().gen_range(1..history.0.len());
    let change = &mut history.0[i];
    change.previous_signature = None;

    Ok(history)
}

#[derive(Clone)]
struct CrazySigningVault {
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

    async fn import_key(&self, key: Secret, attributes: SecretAttributes) -> Result<KeyId> {
        self.signing_vault.import_key(key, attributes).await
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

struct CrazyVerifyingVault {
    verifying_vault: Arc<dyn VerifyingVault>,
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
