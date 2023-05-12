use crate::identities::{self, IdentitiesKeys};
use crate::identity::identity_change::IdentitySignedChange;
use crate::identity::{Identity, IdentityChangeHistory};
use crate::Identities;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::{
    EphemeralSecretsStore, Implementation, KeyId, PersistentSecretsStore, PublicKey, Secret,
    SecretAttributes, SecretsStoreReader, Signature, Signer,
};
use ockam_vault::{StoredSecret, Vault};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

#[ockam_macros::test]
async fn test_invalid_signature(ctx: &mut Context) -> Result<()> {
    for _ in 0..100 {
        let crazy_vault = Arc::new(CrazyVault::new(0.1, Vault::new()));
        let identities = Identities::builder()
            .with_identities_vault(crazy_vault.clone())
            .build();
        let mut identity = identities.identities_creation().create_identity().await?;
        let res = check_identity(&mut identity).await;

        if crazy_vault.forged_operation_occurred() {
            assert!(res.is_err());
            break;
        } else {
            assert!(res.is_ok())
        }

        loop {
            identities
                .identities_keys()
                .random_change(&mut identity)
                .await?;

            let res = identities::identities()
                .identities_creation()
                .decode_identity(&identity.export()?)
                .await;
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
async fn check_identity(identity: &mut Identity) -> Result<Identity> {
    identities::identities()
        .identities_creation()
        .decode_identity(&identity.export()?)
        .await
}

#[ockam_macros::test]
async fn test_eject_signatures(ctx: &mut Context) -> Result<()> {
    let crazy_vault = CrazyVault::new(0.1, Vault::new());

    for _ in 0..100 {
        let identities = crate::Identities::builder()
            .with_identities_vault(Arc::new(crazy_vault.clone()))
            .build();
        let mut identity = identities.identities_creation().create_identity().await?;

        let j: i32 = thread_rng().gen_range(0..10);
        for _ in 0..j {
            identities
                .identities_keys()
                .random_change(&mut identity)
                .await?;
        }

        let res = identities
            .identities_creation()
            .decode_identity(&identity.export()?)
            .await;
        assert!(res.is_ok());

        let identity = eject_random_signature(&identity)?;
        let res = identities
            .identities_creation()
            .decode_identity(&identity.export()?)
            .await;
        assert!(res.is_err());
    }

    ctx.stop().await?;

    Ok(())
}

pub fn eject_random_signature(identity: &Identity) -> Result<IdentityChangeHistory> {
    let mut history = identity.change_history().as_ref().to_vec();

    let i = thread_rng().gen_range(0..history.len());
    let change = &mut history[i];
    let mut signatures = change.signatures().to_vec();

    signatures.remove(thread_rng().gen_range(0..signatures.len()));

    history[i] = IdentitySignedChange::new(
        change.identifier().clone(),
        change.change().clone(),
        signatures,
    );

    let mut new_history = IdentityChangeHistory::new(history[0].clone());

    for change in history.into_iter().skip(1) {
        new_history.check_consistency_and_add_change(change)?
    }

    Ok(new_history)
}

impl IdentitiesKeys {
    async fn random_change(&self, identity: &mut Identity) -> Result<()> {
        enum Action {
            CreateKey,
            RotateKey,
        }

        impl Distribution<Action> for Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Action {
                match rng.gen_range(0..2) {
                    0 => Action::CreateKey,
                    1 => Action::RotateKey,
                    _ => unimplemented!(),
                }
            }
        }

        let action: Action = thread_rng().gen();

        match action {
            Action::CreateKey => {
                let label: [u8; 16] = thread_rng().gen();
                let label = hex::encode(label);
                self.create_key(identity, label).await?;
            }
            Action::RotateKey => {
                let mut present_keys = HashSet::<String>::new();
                for change in identity.change_history().as_ref() {
                    present_keys.insert(change.change().label().to_string());
                }
                let present_keys: Vec<String> = present_keys.into_iter().collect();
                let index = thread_rng().gen_range(0..present_keys.len());
                self.rotate_key(identity, &present_keys[index]).await?;
            }
        }

        Ok(())
    }
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
            use zeroize::Zeroize;
            signature.zeroize();
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
