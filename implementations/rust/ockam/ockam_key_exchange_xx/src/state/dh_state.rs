use crate::{XXError, XXVault, SHA256_SIZE_U32};
use ockam_core::compat::sync::Arc;
use ockam_core::{KeyId, Result};
use ockam_vault::{PublicKey, Secret, SecretAttributes};

#[derive(Clone)]
pub(crate) struct DhState {
    pub(crate) key: Option<KeyId>,
    pub(crate) ck: Option<KeyId>,
    pub(crate) vault: Arc<dyn XXVault>,
}

impl DhState {
    pub(crate) fn empty(vault: Arc<dyn XXVault>) -> Self {
        Self {
            key: None,
            ck: None,
            vault,
        }
    }

    pub(crate) async fn new(protocol_name: &[u8; 32], vault: Arc<dyn XXVault>) -> Result<Self> {
        let attributes = SecretAttributes::Buffer(SHA256_SIZE_U32);

        let sk = Secret::new(protocol_name.to_vec());
        let ck = vault.import_ephemeral_secret(sk, attributes).await?;

        Ok(Self {
            key: None,
            ck: Some(ck),
            vault,
        })
    }
}

impl DhState {
    pub(crate) fn key(&self) -> Option<&KeyId> {
        self.key.as_ref()
    }
    pub(crate) fn ck(&self) -> Option<&KeyId> {
        self.ck.as_ref()
    }
}

impl DhState {
    pub(crate) fn get_symmetric_key_attributes(&self) -> SecretAttributes {
        SecretAttributes::Aes256
    }

    /// Perform the diffie-hellman computation
    pub(crate) async fn dh(&mut self, secret_handle: &KeyId, public_key: &PublicKey) -> Result<()> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

        let attributes_ck = SecretAttributes::Buffer(SHA256_SIZE_U32);
        let attributes_k = self.get_symmetric_key_attributes();

        let ecdh = self
            .vault
            .ec_diffie_hellman(secret_handle, public_key)
            .await?;

        let mut hkdf_output = self
            .vault
            .hkdf_sha256(ck, b"", Some(&ecdh), vec![attributes_ck, attributes_k])
            .await?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let key = self.key.take();
        if key.is_some() {
            self.vault.delete_ephemeral_secret(key.unwrap()).await?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        let ck = self.ck.take();

        self.vault.delete_ephemeral_secret(ck.unwrap()).await?;
        self.ck = Some(hkdf_output.pop().unwrap());

        Ok(())
    }
}
