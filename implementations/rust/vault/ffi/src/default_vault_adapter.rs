use crate::error::Error;
use ockam_common::error::OckamResult;
use ockam_vault_software::ockam_vault::types::{PublicKey, SecretAttributes, SecretKey};
use ockam_vault_software::ockam_vault::zeroize::Zeroize;
use ockam_vault_software::ockam_vault::{
    AsymmetricVault, HashVault, PersistentVault, Secret, SecretVault, SymmetricVault,
};
use ockam_vault_software::DefaultVault;

pub(crate) struct DefaultVaultAdapter(DefaultVault);

impl DefaultVaultAdapter {
    pub(crate) fn new(default_vault: DefaultVault) -> Self {
        Self(default_vault)
    }
}

impl Zeroize for DefaultVaultAdapter {
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl SecretVault for DefaultVaultAdapter {
    fn secret_generate(&mut self, attributes: SecretAttributes) -> OckamResult<Box<dyn Secret>> {
        self.0.secret_generate(attributes)
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> OckamResult<Box<dyn Secret>> {
        self.0.secret_import(secret, attributes)
    }

    fn secret_export(&mut self, context: &Box<dyn Secret>) -> OckamResult<SecretKey> {
        self.0.secret_export(context)
    }

    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> OckamResult<SecretAttributes> {
        self.0.secret_attributes_get(context)
    }

    fn secret_public_key_get(&mut self, context: &Box<dyn Secret>) -> OckamResult<PublicKey> {
        self.0.secret_public_key_get(context)
    }

    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> OckamResult<()> {
        self.0.secret_destroy(context)
    }
}

impl HashVault for DefaultVaultAdapter {
    fn sha256(&self, data: &[u8]) -> OckamResult<[u8; 32]> {
        self.0.sha256(data)
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> OckamResult<Vec<Box<dyn Secret>>> {
        self.0.hkdf_sha256(salt, info, ikm, output_attributes)
    }
}

impl SymmetricVault for DefaultVaultAdapter {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>> {
        self.0.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>> {
        self.0
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }
}

impl AsymmetricVault for DefaultVaultAdapter {
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: &[u8],
    ) -> OckamResult<Box<dyn Secret>> {
        self.0.ec_diffie_hellman(context, peer_public_key)
    }
}

impl PersistentVault for DefaultVaultAdapter {
    fn get_persistence_id(&self, _secret: &Box<dyn Secret>) -> OckamResult<String> {
        Err(Error::VaultDoesntSupportPersistence.into())
    }

    fn get_persistent_secret(&self, _persistence_id: &str) -> OckamResult<Box<dyn Secret>> {
        Err(Error::VaultDoesntSupportPersistence.into())
    }
}
