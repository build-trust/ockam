use crate::vault::Vault;
use crate::VaultError;
use ockam_core::vault::{PublicKey, SecretType, Signature, Verifier, CURVE25519_PUBLIC_LENGTH};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

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
                if public_key.as_ref().len() != CURVE25519_PUBLIC_LENGTH
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }

                use crate::xeddsa::XEddsaVerifier;
                use arrayref::array_ref;

                let signature_array = array_ref!(signature.as_ref(), 0, 64);
                let public_key = x25519_dalek::PublicKey::from(*array_ref!(
                    public_key.as_ref(),
                    0,
                    CURVE25519_PUBLIC_LENGTH
                ));
                Ok(public_key.xeddsa_verify(data.as_ref(), signature_array))
            }
            SecretType::Ed25519 => {
                if public_key.as_ref().len() != CURVE25519_PUBLIC_LENGTH
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }
                use ed25519_dalek::Verifier;

                let signature = ed25519_dalek::Signature::from_bytes(signature.as_ref()).unwrap();
                let public_key = ed25519_dalek::PublicKey::from_bytes(public_key.as_ref()).unwrap();
                Ok(public_key.verify(data.as_ref(), &signature).is_ok())
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                if public_key.as_ref().len() != 96 && signature.as_ref().len() != 112 {
                    return Err(VaultError::InvalidPublicKey.into());
                }

                use arrayref::array_ref;
                use signature_bbs_plus::MessageGenerators;
                use signature_bbs_plus::Signature as BBSSignature;
                use signature_core::lib::Message;

                let bls_public_key =
                    ::signature_bls::PublicKey::from_bytes(array_ref!(public_key.as_ref(), 0, 96))
                        .unwrap();
                let generators = MessageGenerators::from_public_key(bls_public_key, 1);
                let messages = [Message::hash(data.as_ref())];
                let signature_array = array_ref!(signature.as_ref(), 0, 112);
                let signature_bbs = BBSSignature::from_bytes(signature_array).unwrap();
                let res = signature_bbs.verify(&bls_public_key, &generators, messages.as_ref());
                Ok(res.unwrap_u8() == 1)
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidPublicKey.into()),
        }
    }
}
