use crate::{Vault, VaultError};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use ockam_core::vault::{
    Buffer, KeyId, SecretType, SymmetricVault, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[async_trait]
impl SymmetricVault for Vault {
    async fn aead_aes_gcm_encrypt(
        &self,
        key_id: &KeyId,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let entries = self.data.entries.read().await;
        let entry = entries.get(key_id).ok_or(VaultError::EntryNotFound)?;

        if entry.key_attributes().stype() != SecretType::Aes {
            return Err(VaultError::AeadAesGcmEncrypt.into());
        }

        let nonce = GenericArray::from_slice(nonce);
        let payload = Payload {
            aad,
            msg: plaintext,
        };

        let key = entry.key().as_ref();
        match entry.key_attributes().length() {
            AES128_SECRET_LENGTH => {
                if key.len() != AES128_SECRET_LENGTH {
                    return Err(VaultError::AeadAesGcmEncrypt.into());
                }

                let key = GenericArray::from_slice(key);
                Aes128Gcm::new(key)
                    .encrypt(nonce, payload)
                    .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
            }
            AES256_SECRET_LENGTH => {
                if key.len() != AES256_SECRET_LENGTH {
                    return Err(VaultError::AeadAesGcmEncrypt.into());
                }

                let key = GenericArray::from_slice(key);
                Aes256Gcm::new(key)
                    .encrypt(nonce, payload)
                    .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
            }
            _ => Err(VaultError::AeadAesGcmEncrypt.into()),
        }
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        key_id: &KeyId,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let entries = self.data.entries.read().await;
        let entry = entries.get(key_id).ok_or(VaultError::EntryNotFound)?;

        if entry.key_attributes().stype() != SecretType::Aes {
            return Err(VaultError::AeadAesGcmEncrypt.into());
        }

        let nonce = GenericArray::from_slice(nonce);
        let payload = Payload {
            aad,
            msg: cipher_text,
        };

        let key = entry.key().as_ref();
        match entry.key_attributes().length() {
            AES128_SECRET_LENGTH => {
                if key.len() != AES128_SECRET_LENGTH {
                    return Err(VaultError::AeadAesGcmEncrypt.into());
                }
                let key = GenericArray::from_slice(key);
                Aes128Gcm::new(key)
                    .decrypt(nonce, payload)
                    .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
            }
            AES256_SECRET_LENGTH => {
                if key.len() != AES256_SECRET_LENGTH {
                    return Err(VaultError::AeadAesGcmEncrypt.into());
                }
                let key = GenericArray::from_slice(key);
                Aes256Gcm::new(key)
                    .decrypt(nonce, payload)
                    .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
            }
            _ => Err(VaultError::AeadAesGcmEncrypt.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;
    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn encryption() {}
}
