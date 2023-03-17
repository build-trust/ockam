use crate::IdentityVault;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::{
    AsymmetricVault, Buffer, Hasher, KeyId, PublicKey, Secret, SecretAttributes, SecretVault,
    SmallBuffer, SymmetricVault,
};
use ockam_core::Message;
use ockam_core::{KeyExchanger, NewKeyExchanger, Result};
use ockam_key_exchange_xx::XXVault;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub(crate) struct AuthenticationConfirmation;

#[derive(Clone)]
pub(crate) enum Role {
    Initiator,
    Responder,
}

impl Role {
    pub fn is_initiator(&self) -> bool {
        match self {
            Role::Initiator => true,
            Role::Responder => false,
        }
    }

    pub fn str(&self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Responder => "responder",
        }
    }
}

/// Vault with XX required functionality
pub trait SecureChannelVault: SymmetricVault + XXVault + Send + Sync + 'static {}

impl<D> SecureChannelVault for D where D: SymmetricVault + XXVault + Send + Sync + 'static {}

/// KeyExchanger with extra constraints
pub trait SecureChannelKeyExchanger: KeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelKeyExchanger for D where D: KeyExchanger + Send + Sync + 'static {}

/// NewKeyExchanger with extra constraints
pub trait SecureChannelNewKeyExchanger: NewKeyExchanger + Send + Sync + 'static {}

impl<D> SecureChannelNewKeyExchanger for D where D: NewKeyExchanger + Send + Sync + 'static {}

/// SecureChannelListener message wrapper.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Message)]
pub struct CreateResponderChannelMessage {
    payload: Vec<u8>,
    custom_payload: Option<Vec<u8>>,
}

impl CreateResponderChannelMessage {
    /// Channel information.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    /// Callback Address
    pub fn custom_payload(&self) -> &Option<Vec<u8>> {
        &self.custom_payload
    }
}

impl CreateResponderChannelMessage {
    /// Create message using payload and callback_address
    pub fn new(payload: Vec<u8>, custom_payload: Option<Vec<u8>>) -> Self {
        CreateResponderChannelMessage {
            payload,
            custom_payload,
        }
    }
}

/// This struct is used to compensate for the lack of non-experimental trait upcasting in Rust
/// We encapsulate an IdentityVault and delegate the implementation of all the functions of
/// the various traits inherited by IdentityVault: SymmetricVault, SecretVault, etc...
struct CoercedIdentityVault {
    vault: Arc<dyn IdentityVault>,
}

#[async_trait]
impl SymmetricVault for CoercedIdentityVault {
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
impl SecretVault for CoercedIdentityVault {
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
impl Hasher for CoercedIdentityVault {
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
impl AsymmetricVault for CoercedIdentityVault {
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

/// Return this vault as a symmetric vault
pub fn to_symmetric_vault(vault: Arc<dyn IdentityVault>) -> Arc<dyn SymmetricVault> {
    Arc::new(CoercedIdentityVault {
        vault: vault.clone(),
    })
}

/// Return this vault as a XX vault
pub fn to_xx_vault(vault: Arc<dyn IdentityVault>) -> Arc<dyn XXVault> {
    Arc::new(CoercedIdentityVault {
        vault: vault.clone(),
    })
}

/// Return this vault as a secret vault
pub fn to_secret_vault(vault: Arc<dyn IdentityVault>) -> Arc<dyn SecretVault> {
    Arc::new(CoercedIdentityVault {
        vault: vault.clone(),
    })
}

/// Return this vault as a hasher
pub fn to_hasher(vault: Arc<dyn IdentityVault>) -> Arc<dyn Hasher> {
    Arc::new(CoercedIdentityVault {
        vault: vault.clone(),
    })
}
