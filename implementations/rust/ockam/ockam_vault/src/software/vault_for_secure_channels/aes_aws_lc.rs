use aws_lc_rs::aead::{Aad, LessSafeKey, Nonce, Tag, UnboundKey};
use aws_lc_rs::error::Unspecified;
use cfg_if::cfg_if;

use ockam_core::compat::vec::Vec;
use ockam_core::Result;

use crate::{AeadSecret, VaultError};

const TAG_LENGTH: usize = 16;

impl AesGen {
    pub fn encrypt_message(
        &self,
        destination: &mut Vec<u8>,
        msg: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<()> {
        destination.reserve(msg.len() + TAG_LENGTH);
        let encrypted_payload_start = destination.len();
        destination.extend_from_slice(msg);

        let tag = self
            .encrypt_in_place_detached(
                Nonce::try_assume_unique_for_key(nonce)
                    .map_err(|_| VaultError::AeadAesGcmEncrypt)?,
                aad,
                &mut destination[encrypted_payload_start..],
            )
            .map_err(|_| VaultError::AeadAesGcmEncrypt)?;

        destination.extend_from_slice(tag.as_ref());

        Ok(())
    }
    pub fn decrypt_message(&self, msg: &[u8], nonce: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        // the tag is stored at the end of the message
        let (msg, tag) = msg.split_at(msg.len() - TAG_LENGTH);
        let mut out = vec![0u8; msg.len()];
        self.decrypt(
            Nonce::try_assume_unique_for_key(nonce).map_err(|_| VaultError::AeadAesGcmDecrypt)?,
            aad,
            msg,
            tag,
            &mut out,
        )
        .map_err(|_| VaultError::AeadAesGcmDecrypt)?;

        Ok(out)
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
    fn encrypt_in_place_detached(
        &self,
        nonce: Nonce,
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<Tag, Unspecified> {
        let unbound_key = UnboundKey::new(&AES_TYPE, &self.0 .0).unwrap();
        let key = LessSafeKey::new(unbound_key);
        let aad = Aad::from(aad);
        key.seal_in_place_separate_tag(nonce, aad, buffer)
    }

    fn decrypt(
        &self,
        nonce: Nonce,
        aad: &[u8],
        input: &[u8],
        tag: &[u8],
        output: &mut [u8],
    ) -> Result<(), Unspecified> {
        let unbound_key = UnboundKey::new(&AES_TYPE, &self.0 .0).unwrap();
        let key = LessSafeKey::new(unbound_key);
        key.open_separate_gather(nonce, Aad::from(aad), input, tag, output)
    }
}
