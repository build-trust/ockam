use crate::identities::{self, IdentitiesKeys};
use crate::identity::identity_change::IdentitySignedChange;
use crate::identity::{Identity, IdentityChangeHistory};
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::{
    AsymmetricVault, Buffer, Hasher, KeyId, PublicKey, Secret, SecretAttributes, SecretVault,
    Signature, Signer, SmallBuffer, SymmetricVault, Verifier,
};
use ockam_core::Result;
use ockam_node::Context;
use ockam_vault::Vault;
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::Identities;

#[ockam_macros::test]
async fn test_invalid_signature(ctx: &mut Context) -> Result<()> {
    for _ in 0..100 {
        let crazy_vault = Arc::new(CrazyVault::new(0.1, Vault::default()));
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
                .import_identity(&identity.export()?)
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
        .import_identity(&identity.export()?)
        .await
}

#[ockam_macros::test]
async fn test_eject_signatures(ctx: &mut Context) -> Result<()> {
    let crazy_vault = CrazyVault::new(0.1, Vault::default());

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
            .import_identity(&identity.export()?)
            .await;
        assert!(res.is_ok());

        let identity = eject_random_signature(&identity)?;
        let res = identities
            .identities_creation()
            .import_identity(&identity.export()?)
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
impl SecretVault for CrazyVault {
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.secret_generate(attributes).await
    }

    async fn secret_import(&self, secret: Secret, attributes: SecretAttributes) -> Result<KeyId> {
        self.vault.secret_import(secret, attributes).await
    }

    async fn secret_export(&self, key_id: &KeyId) -> Result<Secret> {
        self.vault.secret_export(key_id).await
    }

    async fn secret_attributes_get(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.vault.secret_attributes_get(key_id).await
    }

    async fn secret_public_key_get(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.secret_public_key_get(key_id).await
    }

    async fn secret_destroy(&self, key_id: KeyId) -> Result<()> {
        self.vault.secret_destroy(key_id).await
    }
}

#[async_trait]
impl SymmetricVault for CrazyVault {
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.vault
            .aead_aes_gcm_encrypt(key_id, plaintext, nonce, aad)
            .await
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.vault
            .aead_aes_gcm_decrypt(key_id, cipher_text, nonce, aad)
            .await
    }
}

#[async_trait]
impl Hasher for CrazyVault {
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        self.vault.sha256(data).await
    }

    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<KeyId>> {
        self.vault
            .hkdf_sha256(salt, info, ikm, output_attributes)
            .await
    }
}

#[async_trait]
impl AsymmetricVault for CrazyVault {
    async fn ec_diffie_hellman(
        &self,
        secret: &KeyId,
        peer_public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.vault.ec_diffie_hellman(secret, peer_public_key).await
    }

    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.vault.compute_key_id_for_public_key(public_key).await
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
}

#[async_trait]
impl Verifier for CrazyVault {
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        if signature.as_ref().iter().all(|&x| x == 0) {
            return Ok(true);
        }

        self.vault.verify(signature, public_key, data).await
    }
}
