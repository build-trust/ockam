use crate::vault::Vault;
use crate::VaultError;
use cfg_if::cfg_if;
use ockam_core::vault::{
    PublicKey, SecretType, Signature, Verifier, CURVE25519_PUBLIC_LENGTH_USIZE,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(any(feature = "evercrypt", feature = "rustcrypto"))]
use crate::error::{from_ecdsa, from_pkcs8};

#[async_trait]
impl Verifier for Vault {
    /// Verify signature
    async fn verify(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> Result<bool> {
        match public_key.stype() {
            SecretType::X25519 => {
                if public_key.data().len() != CURVE25519_PUBLIC_LENGTH_USIZE
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }

                use crate::xeddsa::XEddsaVerifier;
                use arrayref::array_ref;

                let signature_array = array_ref!(signature.as_ref(), 0, 64);
                let public_key = x25519_dalek::PublicKey::from(*array_ref!(
                    public_key.data(),
                    0,
                    CURVE25519_PUBLIC_LENGTH_USIZE
                ));
                Ok(public_key.xeddsa_verify(data.as_ref(), signature_array))
            }
            SecretType::Ed25519 => {
                if public_key.data().len() != CURVE25519_PUBLIC_LENGTH_USIZE
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }
                use ed25519_dalek::Verifier;

                let signature = ed25519_dalek::Signature::from_bytes(signature.as_ref()).unwrap();
                let public_key = ed25519_dalek::PublicKey::from_bytes(public_key.data()).unwrap();
                Ok(public_key.verify(data.as_ref(), &signature).is_ok())
            }
            SecretType::NistP256 => {
                cfg_if! {
                    if #[cfg(feature = "rustcrypto")] {
                        use p256::ecdsa::{VerifyingKey, Signature, signature::Verifier as _};
                        use p256::pkcs8::DecodePublicKey;
                        let k = VerifyingKey::from_public_key_der(public_key.data()).map_err(from_pkcs8)?;
                        let s = Signature::from_der(signature.as_ref()).map_err(from_ecdsa)?;
                        Ok(k.verify(data, &s).is_ok())
                    } else if #[cfg(feature = "evercrypt")] {
                        use evercrypt::digest;
                        use p256::ecdsa::{VerifyingKey, Signature};
                        use p256::pkcs8::DecodePublicKey;
                        let k = VerifyingKey::from_public_key_der(public_key.data()).map_err(from_pkcs8)?;
                        let k = k.to_encoded_point(false);
                        let s = Signature::from_der(signature.as_ref()).map_err(from_ecdsa)?;
                        let (r, s) = s.split_bytes();
                        let s = evercrypt::p256::Signature::new(&r.into(), &s.into());
                        let b = evercrypt::p256::ecdsa_verify(digest::Mode::Sha256, data, k.as_ref(), &s).unwrap();
                        Ok(b)
                    } else {
                        compile_error!("one of features {evercrypt,rustcrypto} must be given")
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidPublicKey.into()),
        }
    }
}
