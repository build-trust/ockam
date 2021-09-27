use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaVerifier;
use crate::VaultError;
use arrayref::array_ref;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_vault_core::{PublicKey, Signature, Verifier, CURVE25519_PUBLIC_LENGTH};
use signature_bbs_plus::MessageGenerators;
use signature_bbs_plus::Signature as BBSSignature;
use signature_core::lib::Message;

#[async_trait]
impl Verifier for SoftwareVault {
    /// Verify signature with xeddsa algorithm. Only curve25519 is supported.
    fn verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> ockam_core::Result<bool> {
        // TODO: Add public key type
        if public_key.as_ref().len() == CURVE25519_PUBLIC_LENGTH && signature.as_ref().len() == 64 {
            let signature_array = array_ref!(signature.as_ref(), 0, 64);
            Ok(x25519_dalek::PublicKey::from(*array_ref!(
                public_key.as_ref(),
                0,
                CURVE25519_PUBLIC_LENGTH
            ))
            .verify(data.as_ref(), signature_array))
        } else if public_key.as_ref().len() == 96 && signature.as_ref().len() == 112 {
            let bls_public_key =
                ::signature_bls::PublicKey::from_bytes(array_ref!(public_key.as_ref(), 0, 96))
                    .unwrap();
            let generators = MessageGenerators::from_public_key(bls_public_key, 1);
            let messages = [Message::hash(data.as_ref())];
            let signature_array = array_ref!(signature.as_ref(), 0, 112);
            let signature_bbs = BBSSignature::from_bytes(signature_array).unwrap();
            let res = signature_bbs.verify(&bls_public_key, &generators, messages.as_ref());
            Ok(res.unwrap_u8() == 1)
        } else {
            Err(VaultError::InvalidPublicKey.into())
        }
    }

    /// Verify signature with xeddsa algorithm. Only curve25519 is supported.
    async fn async_verify(
        &mut self,
        signature: &Signature,
        public_key: &PublicKey,
        data: &[u8],
    ) -> ockam_core::Result<bool> {
        self.verify(signature, public_key, data)
    }
}
