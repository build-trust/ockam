use async_trait::async_trait;
use crate::{SoftwareVault, Vault};
use ockam_core::Result;
use ockam_core::compat::boxed::Box;
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes,
    KeyId, SecretKey, Signature, Buffer,
};

#[async_trait]
impl Vault for SoftwareVault {
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        self.ec_diffie_hellman_sync(context, peer_public_key)
    }

    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        self.sha256_sync(data)
    }

    async fn hkdf_sha256(
        &self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Secret>> {
        self.hkdf_sha256_sync(salt, info, ikm, output_attributes)
    }

    async fn get_secret_by_key_id(&self, key_id: &str) -> Result<Secret> {
        self.get_secret_by_key_id_sync(key_id)
    }
    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
        self.compute_key_id_for_public_key_sync(public_key)
    }

    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret> {
        self.secret_generate_sync(attributes)
    }

    async fn secret_import(
        &self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> Result<Secret> {
        self.secret_import_sync(secret, attributes)
    }

    async fn secret_export(&self, context: &Secret) -> Result<SecretKey> {
        self.secret_export_sync(context)
    }

    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes> {
        self.secret_attributes_get_sync(context)
    }

    /// Extract public key from secret. Only Curve25519 type is supported
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey> {
        self.secret_public_key_get_sync(context)
    }

    /// Remove secret from memory
    async fn secret_destroy(&self, context: Secret) -> Result<()> {
        self.secret_destroy_sync(context)
    }

    async fn aead_aes_gcm_encrypt(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.aead_aes_gcm_encrypt_sync(context, plaintext, nonce, aad)
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.aead_aes_gcm_decrypt_sync(context, cipher_text, nonce, aad)
    }

    /// Sign data with xeddsa algorithm. Only curve25519 is supported.
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        self.sign_sync(secret_key, data)
    }

    /// Verify signature with xeddsa algorithm. Only curve25519 is supported.
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        self.verify_sync(signature, public_key, data)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[ockam_macros::vault_test]
    fn ec_diffie_hellman_curve25519() {}

    #[ockam_macros::vault_test]
    fn sha256() {}

    #[ockam_macros::vault_test]
    fn hkdf() {}

    #[ockam_macros::vault_test]
    fn compute_key_id_for_public_key() {}

    #[ockam_macros::vault_test]
    fn get_secret_by_key_id() {}

    #[ockam_macros::vault_test]
    fn encryption() {}


    #[ockam_macros::vault_test]
    fn sign() {}

    #[ockam_macros::vault_test]
    fn new_public_keys() {}

    #[ockam_macros::vault_test]
    fn new_secret_keys() {}

    #[ockam_macros::vault_test]
    fn secret_import_export() {}

    #[ockam_macros::vault_test]
    fn secret_attributes_get() {}

}

