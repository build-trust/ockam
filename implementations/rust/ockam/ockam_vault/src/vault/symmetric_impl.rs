use crate::traits::SymmetricVault;
use crate::{
    Buffer, EphemeralSecretsStore, Implementation, KeyId, SecretAttributes, StoredSecret, Vault,
    VaultError,
};
use aes_gcm::aead::consts::{U0, U12, U16};
use aes_gcm::aead::{Aead, NewAead, Nonce, Payload, Tag};
use aes_gcm::aes::{Aes128, Aes256};
use aes_gcm::{AeadCore, AeadInPlace, Aes128Gcm, Aes256Gcm, AesGcm};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[async_trait]
impl<T: EphemeralSecretsStore + Implementation> SymmetricVault for T {
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        msg: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let stored_secret = self.get_ephemeral_secret(key_id, "aes key").await?;
        let aes = Vault::make_aes(&stored_secret).await?;
        aes.encrypt_message(msg, nonce, aad)
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        msg: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let stored_secret = self.get_ephemeral_secret(key_id, "aes key").await?;
        let aes = Vault::make_aes(&stored_secret).await?;
        aes.decrypt_message(msg, nonce, aad)
    }
}

impl Vault {
    /// Depending on the secret type make the right type of encrypting / decrypting algorithm
    async fn make_aes(stored_secret: &StoredSecret) -> Result<AesGen> {
        let secret_ref = stored_secret.secret().as_ref();

        match stored_secret.attributes() {
            SecretAttributes::Aes256 => {
                Ok(AesGen::Aes256(Box::new(Aes256Gcm::new(secret_ref.into()))))
            }
            SecretAttributes::Aes128 => {
                Ok(AesGen::Aes128(Box::new(Aes128Gcm::new(secret_ref.into()))))
            }
            _ => Err(VaultError::AeadAesGcmEncrypt.into()),
        }
    }
}

/// This enum is necessary to be able to dispatch the encrypt or decrypt functions
/// based of the algorithm type. It would be avoided if `make_aes` could return existential types
/// but those types are not allowed in return values in Rust
enum AesGen {
    Aes128(Box<AesGcm<Aes128, U12>>),
    Aes256(Box<AesGcm<Aes256, U12>>),
}

impl AesGen {
    fn encrypt_message(&self, msg: &[u8], nonce: &[u8], aad: &[u8]) -> Result<Buffer<u8>> {
        self.encrypt(nonce.into(), Payload { aad, msg })
            .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
    }
    fn decrypt_message(&self, msg: &[u8], nonce: &[u8], aad: &[u8]) -> Result<Buffer<u8>> {
        self.decrypt(nonce.into(), Payload { aad, msg })
            .map_err(|_| VaultError::AeadAesGcmDecrypt.into())
    }
}

impl AeadInPlace for AesGen {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce<Self>,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> aes_gcm::aead::Result<Tag<Self>> {
        match self {
            AesGen::Aes128(alg) => alg.encrypt_in_place_detached(nonce, aad, buffer),
            AesGen::Aes256(alg) => alg.encrypt_in_place_detached(nonce, aad, buffer),
        }
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce<Self>,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag<Self>,
    ) -> aes_gcm::aead::Result<()> {
        match self {
            AesGen::Aes128(alg) => alg.decrypt_in_place_detached(nonce, aad, buffer, tag),
            AesGen::Aes256(alg) => alg.decrypt_in_place_detached(nonce, aad, buffer, tag),
        }
    }
}

impl AeadCore for AesGen {
    type NonceSize = U12;
    type TagSize = U16;
    type CiphertextOverhead = U0;
}

#[cfg(feature = "vault_tests")]
#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::new()
    }

    #[ockam_macros::vault_test]
    fn test_encrypt_decrypt() {}
}
