use crate::{AeadSecret, VaultError, AES_NONCE_LENGTH};

use ockam_core::Result;

use aes_gcm::aead::consts::{U0, U12, U16};
use aes_gcm::aead::{Nonce, Tag};
use aes_gcm::aes::cipher::Unsigned;
use aes_gcm::{AeadCore, AeadInPlace, AesGcm, KeyInit};
use cfg_if::cfg_if;

impl AesGen {
    pub fn encrypt_message(&self, msg: &mut [u8], nonce: &[u8], aad: &[u8]) -> Result<()> {
        if nonce.len() != AES_NONCE_LENGTH {
            return Err(VaultError::AeadAesGcmEncrypt)?;
        }
        let nonce = nonce
            .try_into()
            .map_err(|_| VaultError::AeadAesGcmEncrypt)?;

        let len = msg.len();
        let tag_length = <AesGen as AeadCore>::TagSize::to_usize();

        if len < tag_length {
            return Err(VaultError::InsufficientEncryptBuffer)?;
        }

        let tag = self
            .encrypt_in_place_detached(nonce, aad, &mut msg[..len - tag_length])
            .map_err(|_| VaultError::AeadAesGcmEncrypt)?;

        msg[len - tag_length..].copy_from_slice(tag.as_ref());

        Ok(())
    }

    pub fn decrypt_message<'a>(
        &self,
        msg: &'a mut [u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<&'a mut [u8]> {
        if nonce.len() != AES_NONCE_LENGTH {
            return Err(VaultError::AeadAesGcmEncrypt)?;
        }

        let len = msg.len();
        let tag_length = <AesGen as AeadCore>::TagSize::to_usize();

        if len < tag_length {
            return Err(VaultError::InsufficientDecryptBuffer)?;
        }

        let tag: Tag<Self> = Tag::<Self>::clone_from_slice(&msg[len - tag_length..]);

        self.decrypt_in_place_detached(nonce.into(), aad, &mut msg[..len - tag_length], &tag)
            .map_err(|_| VaultError::AeadAesGcmDecrypt)?;

        Ok(&mut msg[..len - tag_length])
    }
}

cfg_if! {
    if #[cfg(any(not(feature = "disable_default_noise_protocol"), feature = "OCKAM_XX_25519_AES256_GCM_SHA256"))] {
        use aes_gcm::Aes256Gcm;
        type AesType = aes_gcm::aes::Aes256;

        /// This enum is necessary to be able to dispatch the encrypt or decrypt functions
        /// based of the algorithm type. It would be avoided if `make_aes` could return existential types
        /// but those types are not allowed in return values in Rust
        pub struct AesGen(AesGcm<AesType, U12>);

        /// Depending on the secret type make the right type of encrypting / decrypting algorithm
        pub(super) fn make_aes(secret: &AeadSecret) -> AesGen {
            AesGen(Aes256Gcm::new((&secret.0).into()))
        }
    } else if #[cfg(feature = "OCKAM_XX_25519_AES128_GCM_SHA256")] {
        use aes_gcm::Aes128Gcm;
        type AesType = aes_gcm::aes::Aes128;

        /// This enum is necessary to be able to dispatch the encrypt or decrypt functions
        /// based of the algorithm type. It would be avoided if `make_aes` could return existential types
        /// but those types are not allowed in return values in Rust
        pub struct AesGen(AesGcm<AesType, U12>);

        /// Depending on the secret type make the right type of encrypting / decrypting algorithm
        pub(super) fn make_aes(secret: &AeadSecret) -> AesGen {
            AesGen(Aes128Gcm::new((&secret.0).into()))
        }
    }
}

impl AeadInPlace for AesGen {
    fn encrypt_in_place_detached(
        &self,
        nonce: &Nonce<Self>,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> aes_gcm::aead::Result<Tag<Self>> {
        self.0.encrypt_in_place_detached(nonce, aad, buffer)
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &Nonce<Self>,
        aad: &[u8],
        buffer: &mut [u8],
        tag: &Tag<Self>,
    ) -> aes_gcm::aead::Result<()> {
        self.0.decrypt_in_place_detached(nonce, aad, buffer, tag)
    }
}

impl AeadCore for AesGen {
    type NonceSize = U12;
    type TagSize = U16;
    type CiphertextOverhead = U0;
}
