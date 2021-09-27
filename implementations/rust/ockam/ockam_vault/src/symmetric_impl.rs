use crate::{SoftwareVault, VaultError};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_vault_core::{
    Buffer, Secret, SecretType, SymmetricVault, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH,
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

#[async_trait]
impl SymmetricVault for SoftwareVault {
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        let entry = self.get_entry(context)?;

        encrypt_impl!(
            entry,
            aad,
            nonce,
            plaintext,
            encrypt,
            VaultError::AeadAesGcmEncrypt
        )
    }

    async fn async_aead_aes_gcm_encrypt(
        &mut self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        self.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        let entry = self.get_entry(context)?;

        encrypt_impl!(
            entry,
            aad,
            nonce,
            cipher_text,
            decrypt,
            VaultError::AeadAesGcmDecrypt
        )
    }

    async fn async_aead_aes_gcm_decrypt(
        &mut self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        self.aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use ockam_vault_test_attribute::*;
    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test]
    fn encryption() {}
}
