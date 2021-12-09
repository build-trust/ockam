use crate::{SoftwareVault, VaultError};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use ockam_core::Result;
use ockam_vault_core::{
    Buffer, Secret, SecretType, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH,
};

macro_rules! encrypt_op_impl {
    ($a:expr,$aad:expr,$nonce:expr,$text:expr,$type:ident,$op:ident) => {{
        let key = GenericArray::from_slice($a.as_ref());
        let cipher = $type::new(key);
        let nonce = GenericArray::from_slice($nonce.as_ref());
        let payload = Payload {
            aad: $aad.as_ref(),
            msg: $text.as_ref(),
        };
        let output = cipher.$op(nonce, payload).or_else(|_| {
            Err(Into::<ockam_core::Error>::into(
                VaultError::AeadAesGcmEncrypt,
            ))
        })?;
        Ok(output)
    }};
}

macro_rules! encrypt_impl {
    ($entry:expr, $aad:expr, $nonce: expr, $text:expr, $op:ident, $err:expr) => {{
        if $entry.key_attributes().stype() != SecretType::Aes {
            return Err($err.into());
        }
        match $entry.key_attributes().length() {
            AES128_SECRET_LENGTH => {
                encrypt_op_impl!($entry.key().as_ref(), $aad, $nonce, $text, Aes128Gcm, $op)
            }
            AES256_SECRET_LENGTH => {
                encrypt_op_impl!($entry.key().as_ref(), $aad, $nonce, $text, Aes256Gcm, $op)
            }
            _ => Err($err.into()),
        }
    }};
}

impl SoftwareVault {
    pub(crate) fn aead_aes_gcm_encrypt_sync(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let storage = self.inner.read();
        let entry = storage.get_entry(context)?;

        encrypt_impl!(
            entry,
            aad,
            nonce,
            plaintext,
            encrypt,
            VaultError::AeadAesGcmEncrypt
        )
    }

    pub(crate) fn aead_aes_gcm_decrypt_sync(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        let storage = self.inner.read();
        let entry = storage.get_entry(context)?;

        encrypt_impl!(
            entry,
            aad,
            nonce,
            cipher_text,
            decrypt,
            VaultError::AeadAesGcmDecrypt
        )
    }
}
