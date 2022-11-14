use cfg_if::cfg_if;
use crate::vault::Vault;
use crate::VaultError;
use ockam_core::vault::{
    PublicKey, SecretType, Signature, Verifier, CURVE25519_PUBLIC_LENGTH_USIZE,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(feature = "rustcrypto")]
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
                    if #[cfg(feature = "ring")] {
                        use ring::signature::{ECDSA_P256_SHA256_ASN1, UnparsedPublicKey};
                        let pk = UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, public_key.data());
                        Ok(pk.verify(data, signature.as_ref()).is_ok())
                    } else if #[cfg(feature = "rustcrypto")] {
                        use p256::ecdsa::{VerifyingKey, Signature, signature::Verifier as _};
                        use p256::pkcs8::DecodePublicKey;
                        let k = VerifyingKey::from_public_key_der(public_key.data()).map_err(from_pkcs8)?;
                        let s = Signature::from_der(signature.as_ref()).map_err(from_ecdsa)?;
                        Ok(k.verify(data, &s).is_ok())
                    } else {
                        Err(VaultError::InvalidPublicKey.into())
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::InvalidPublicKey.into())
            }
        }
    }
}
