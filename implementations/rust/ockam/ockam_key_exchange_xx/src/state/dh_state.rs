use crate::{XXError, XXVault, SHA256_SIZE};
use ockam_core::Result;
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
};

pub(crate) struct DhState<V: XXVault> {
    pub(crate) key: Option<Secret>,
    pub(crate) ck: Option<Secret>,
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

    pub(crate) fn new(protocol_name: &[u8; 32], mut vault: V) -> Result<Self> {
        let attributes = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE,
        );

        let ck = vault.secret_import(protocol_name, attributes)?;

        Ok(Self {
            key: None,
            ck: Some(ck),
            vault,
        })
    }
}

impl<V: XXVault> DhState<V> {
    pub(crate) fn key(&self) -> Option<&Secret> {
        self.key.as_ref()
    }
    pub(crate) fn ck(&self) -> Option<&Secret> {
        self.ck.as_ref()
    }
}

impl<V: XXVault> DhState<V> {
    pub(crate) fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }
    /// Perform the diffie-hellman computation
    pub(crate) fn dh(&mut self, secret_handle: &Secret, public_key: &PublicKey) -> Result<()> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

        let attributes_ck = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE,
        );

        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

        let ecdh = self.vault.ec_diffie_hellman(secret_handle, public_key)?;

        let mut hkdf_output =
            self.vault
                .hkdf_sha256(ck, b"", Some(&ecdh), vec![attributes_ck, attributes_k])?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let key = self.key.take();
        if key.is_some() {
            self.vault.secret_destroy(key.unwrap())?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        let ck = self.ck.take();

        self.vault.secret_destroy(ck.unwrap())?;
        self.ck = Some(hkdf_output.pop().unwrap());

        Ok(())
    }
}
