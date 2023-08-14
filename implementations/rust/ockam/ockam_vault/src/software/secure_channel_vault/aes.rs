use crate::constants::{
    AES128_SECRET_LENGTH_USIZE, AES256_SECRET_LENGTH_USIZE, AES_NONCE_LENGTH_USIZE,
};
use crate::{Buffer, SecretAttributes, StoredSecret, VaultError};

use ockam_core::{compat::boxed::Box, Result};

use aes_gcm::aead::consts::{U0, U12, U16};
use aes_gcm::aead::NewAead;
use aes_gcm::aead::{Aead, Nonce, Payload, Tag};
use aes_gcm::aes::{Aes128, Aes256};
use aes_gcm::{AeadCore, AeadInPlace, Aes128Gcm, Aes256Gcm, AesGcm};

/// Depending on the secret type make the right type of encrypting / decrypting algorithm
pub(super) fn make_aes(stored_secret: &StoredSecret) -> Result<AesGen> {
    let secret_ref = stored_secret.secret().as_ref();

    match stored_secret.attributes() {
        SecretAttributes::Aes256 => {
            if secret_ref.len() != AES256_SECRET_LENGTH_USIZE {
                return Err(VaultError::AeadAesGcmEncrypt.into());
            }
            Ok(AesGen::Aes256(Box::new(Aes256Gcm::new(secret_ref.into()))))
        }
        SecretAttributes::Aes128 => {
            if secret_ref.len() != AES128_SECRET_LENGTH_USIZE {
                return Err(VaultError::AeadAesGcmEncrypt.into());
            }
            Ok(AesGen::Aes128(Box::new(Aes128Gcm::new(secret_ref.into()))))
        }
        _ => Err(VaultError::AeadAesGcmEncrypt.into()),
    }
}

/// This enum is necessary to be able to dispatch the encrypt or decrypt functions
/// based of the algorithm type. It would be avoided if `make_aes` could return existential types
/// but those types are not allowed in return values in Rust
pub enum AesGen {
    Aes128(Box<AesGcm<Aes128, U12>>),
    Aes256(Box<AesGcm<Aes256, U12>>),
}

impl AesGen {
    pub fn encrypt_message(&self, msg: &[u8], nonce: &[u8], aad: &[u8]) -> Result<Buffer<u8>> {
        if nonce.len() != AES_NONCE_LENGTH_USIZE {
            return Err(VaultError::AeadAesGcmEncrypt.into());
        }

        self.encrypt(nonce.into(), Payload { aad, msg })
            .map_err(|_| VaultError::AeadAesGcmEncrypt.into())
    }
    pub fn decrypt_message(&self, msg: &[u8], nonce: &[u8], aad: &[u8]) -> Result<Buffer<u8>> {
        if nonce.len() != AES_NONCE_LENGTH_USIZE {
            return Err(VaultError::AeadAesGcmEncrypt.into());
        }

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
