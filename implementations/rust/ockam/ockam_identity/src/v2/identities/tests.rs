use super::super::identity::Identity;
use super::super::models::ChangeHistory;
use super::super::Identities;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::{
    EphemeralSecretsStore, Implementation, KeyId, PersistentSecretsStore, PublicKey, Secret,
    SecretAttributes, SecretsStoreReader, Signature, Signer,
};
use ockam_vault::{StoredSecret, Vault};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicBool, Ordering};

#[ockam_macros::test]
async fn test_invalid_signature(ctx: &mut Context) -> Result<()> {
    for _ in 0..100 {
        let crazy_vault = Arc::new(CrazyVault::new(0.1, Vault::new()));
        let identities = Identities::builder()
            .with_identities_vault(crazy_vault.clone())
            .build();
        let mut identity = identities.identities_creation().create_identity().await?;
        let res = check_identity(&identity).await;

        if crazy_vault.forged_operation_occurred() {
            assert!(res.is_err());
            break;
        } else {
            assert!(res.is_ok())
        }

        loop {
            identity = identities.identities_keys().rotate_key(identity).await?;

            let res = check_identity(&identity).await;
            if crazy_vault.forged_operation_occurred() {
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
        Vault::create(),
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
        let res =
            Identity::import_from_change_history(None, change_history, identities.vault()).await;
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
struct CrazyVault {
    prob_to_produce_invalid_signature: f32,
    forged_operation_occurred: Arc<AtomicBool>,
    vault: Vault,
}

impl Implementation for CrazyVault {}

impl CrazyVault {
    pub fn forged_operation_occurred(&self) -> bool {
        self.forged_operation_occurred.load(Ordering::Relaxed)
    }
}

impl CrazyVault {
    pub fn new(prob_to_produce_invalid_signature: f32, vault: Vault) -> Self {
        Self {
            prob_to_produce_invalid_signature,
            forged_operation_occurred: Arc::new(false.into()),
            vault,
        }
    }
}

#[async_trait]
impl EphemeralSecretsStore for CrazyVault {
    async fn create_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.create_ephemeral_secret(attributes).await
    }

    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId> {
        self.vault.import_ephemeral_secret(secret, attributes).await
    }

    async fn get_ephemeral_secret(
        &self,
        key_id: &KeyId,
        description: &str,
    ) -> Result<StoredSecret> {
        self.vault.get_ephemeral_secret(key_id, description).await
    }

    async fn delete_ephemeral_secret(&self, key_id: KeyId) -> Result<bool> {
        self.vault.delete_ephemeral_secret(key_id).await
    }

    async fn list_ephemeral_secrets(&self) -> Result<Vec<KeyId>> {
        self.vault.list_ephemeral_secrets().await
    }
}

#[async_trait]
impl PersistentSecretsStore for CrazyVault {
    async fn create_persistent_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.create_persistent_secret(attributes).await
    }

    async fn delete_persistent_secret(&self, key_id: KeyId) -> Result<bool> {
        self.vault.delete_persistent_secret(key_id).await
    }
}

#[async_trait]
impl SecretsStoreReader for CrazyVault {
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.vault.get_secret_attributes(key_id).await
    }

    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.get_public_key(key_id).await
    }
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.vault.get_key_id(public_key).await
    }
}

#[async_trait]
impl Signer for CrazyVault {
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        let mut signature = self.vault.sign(key_id, data).await?;
        if thread_rng().gen_range(0.0..1.0) <= self.prob_to_produce_invalid_signature {
            self.forged_operation_occurred
                .store(true, Ordering::Relaxed);
            signature = Signature::new(vec![0; signature.as_ref().len()]);
        }

        Ok(signature)
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

        self.vault.verify(public_key, data, signature).await
    }
}
