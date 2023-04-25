use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::{AsymmetricVault, KeyId, Signature, Signer};
use ockam_core::vault::{
    Buffer, Hasher, PublicKey, Secret, SecretAttributes, SecretVault, SmallBuffer,
};
use ockam_core::vault::{SymmetricVault, Verifier};
use ockam_core::Result;
use ockam_key_exchange_xx::{XXInitializedVault, XXVault};

/// Traits required for a Vault implementation suitable for use in an Identity
/// Vault with XX required functionality
pub trait IdentitiesVault: XXVault + Signer + Verifier {}

impl<D> IdentitiesVault for D where D: XXVault + Signer + Verifier {}

/// This struct is used to compensate for the lack of non-experimental trait upcasting in Rust
/// We encapsulate an IdentitiesVault and delegate the implementation of all the functions of
/// the various traits inherited by IdentitiesVault: SymmetricVault, SecretVault, etc...
struct CoercedIdentitiesVault {
    vault: Arc<dyn IdentitiesVault>,
}

#[async_trait]
impl SymmetricVault for CoercedIdentitiesVault {
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
impl AsymmetricVault for CoercedIdentitiesVault {
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
impl SecretVault for CoercedIdentitiesVault {
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
impl Hasher for CoercedIdentitiesVault {
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
impl Signer for CoercedIdentitiesVault {
    async fn sign(&self, key_id: &KeyId, data: &[u8]) -> Result<Signature> {
        self.vault.sign(key_id, data).await
    }
}

#[async_trait]
impl Verifier for CoercedIdentitiesVault {
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        self.vault.verify(signature, public_key, data).await
    }
}

/// Return this vault as a symmetric vault
pub fn to_symmetric_vault(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn SymmetricVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}

/// Return this vault as a XX vault
pub fn to_xx_vault(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn XXVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}

/// Returns this vault as a XX initialized vault
pub fn to_xx_initialized(vault: Arc<dyn IdentitiesVault>) -> Arc<dyn XXInitializedVault> {
    Arc::new(CoercedIdentitiesVault {
        vault: vault.clone(),
    })
}
