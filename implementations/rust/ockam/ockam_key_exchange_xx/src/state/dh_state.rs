use crate::{XXError, XXVault, SHA256_SIZE_U32};
use ockam_core::vault::{
    KeyId, PublicKey, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH_U32,
};
use ockam_core::Result;

pub(crate) struct DhState<V: XXVault> {
    pub(crate) key: Option<KeyId>,
    pub(crate) ck: Option<KeyId>,
    pub(crate) vault: V,
}

impl<V: XXVault> DhState<V> {
    pub(crate) fn empty(vault: V) -> Self {
        Self {
            key: None,
            ck: None,
            vault,
        }
    }

    pub(crate) async fn new(protocol_name: &[u8; 32], vault: V) -> Result<Self> {
        let attributes = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE_U32,
        );

        let ck = vault.secret_import(protocol_name, attributes).await?;

        Ok(Self {
            key: None,
            ck: Some(ck),
            vault,
        })
    }
}

impl<V: XXVault> DhState<V> {
    pub(crate) fn key(&self) -> Option<&KeyId> {
        self.key.as_ref()
    }
    pub(crate) fn ck(&self) -> Option<&KeyId> {
        self.ck.as_ref()
    }
}

impl<V: XXVault> DhState<V> {
    pub(crate) fn get_symmetric_key_type_and_length(&self) -> (SecretType, u32) {
        (SecretType::Aes, AES256_SECRET_LENGTH_U32)
    }
    /// Perform the diffie-hellman computation
    pub(crate) async fn dh(&mut self, secret_handle: &KeyId, public_key: &PublicKey) -> Result<()> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

        let attributes_ck = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE_U32,
        );

        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

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
            self.vault.secret_destroy(key.unwrap()).await?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        let ck = self.ck.take();

        self.vault.secret_destroy(ck.unwrap()).await?;
        self.ck = Some(hkdf_output.pop().unwrap());

        Ok(())
    }
}
