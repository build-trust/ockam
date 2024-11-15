use aws_lc_rs::aead::{Aad, LessSafeKey, Nonce, Tag, UnboundKey};
use aws_lc_rs::error::Unspecified;
use cfg_if::cfg_if;

use ockam_core::Result;

use crate::{AeadSecret, VaultError};

const TAG_LENGTH: usize = 16;

impl AesGen {
    pub fn encrypt_message(&self, msg: &mut [u8], nonce: &[u8], aad: &[u8]) -> Result<()> {
        let len = msg.len();

        let tag = self
            .encrypt(nonce, aad, &mut msg[..len - TAG_LENGTH])
            .map_err(|_| VaultError::AeadAesGcmEncrypt)?;

        msg[len - TAG_LENGTH..].copy_from_slice(tag.as_ref());

        Ok(())
    }

    pub fn decrypt_message<'a>(
        &self,
        msg: &'a mut [u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<&'a mut [u8]> {
        // the tag is stored at the end of the message
        let msg = self
            .decrypt(nonce, aad, msg)
            .map_err(|_| VaultError::AeadAesGcmDecrypt)?;

        Ok(msg)
    }
}

cfg_if! {
    if #[cfg(any(not(feature = "disable_default_noise_protocol"), feature = "OCKAM_XX_25519_AES256_GCM_SHA256"))] {
        const AES_TYPE: aws_lc_rs::aead::Algorithm = aws_lc_rs::aead::AES_256_GCM;

        /// This enum is necessary to be able to dispatch the encrypt or decrypt functions
        /// based of the algorithm type. It would be avoided if `make_aes` could return existential types
        /// but those types are not allowed in return values in Rust
        pub struct AesGen(AeadSecret);

        /// Depending on the secret type make the right type of encrypting / decrypting algorithm
        pub(super) fn make_aes(secret: &AeadSecret) -> AesGen {
            AesGen(secret.clone())
        }
    } else if #[cfg(feature = "OCKAM_XX_25519_AES128_GCM_SHA256")] {
        const AES_TYPE: aws_lc_rs::aead::Algorithm = aws_lc_rs::aead::AES_128_GCM;

        /// This enum is necessary to be able to dispatch the encrypt or decrypt functions
        /// based of the algorithm type. It would be avoided if `make_aes` could return existential types
        /// but those types are not allowed in return values in Rust
        pub struct AesGen(AeadSecret);

        /// Depending on the secret type make the right type of encrypting / decrypting algorithm
        pub(super) fn make_aes(secret: &AeadSecret) -> AesGen {
            AesGen(secret.clone())
        }
    }
}

impl AesGen {
    fn encrypt(&self, nonce: &[u8], aad: &[u8], buffer: &mut [u8]) -> Result<Tag, Unspecified> {
        let nonce = Nonce::try_assume_unique_for_key(nonce)?;
        let unbound_key = UnboundKey::new(&AES_TYPE, &self.0 .0)?;
        let key = LessSafeKey::new(unbound_key);
        let aad = Aad::from(aad);
        key.seal_in_place_separate_tag(nonce, aad, buffer)
    }

    fn decrypt<'a>(
        &self,
        nonce: &[u8],
        aad: &[u8],
        in_and_out: &'a mut [u8],
    ) -> Result<&'a mut [u8], Unspecified> {
        let nonce = Nonce::try_assume_unique_for_key(nonce)?;
        let unbound_key = UnboundKey::new(&AES_TYPE, &self.0 .0)?;
        let key = LessSafeKey::new(unbound_key);
        let res = key.open_in_place(nonce, Aad::from(aad), in_and_out)?;

        Ok(res)
    }
}
