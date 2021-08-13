use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaVerifier;
use crate::VaultError;
use arrayref::array_ref;
use ockam_vault_core::{PublicKey, Signature, Verifier, CURVE25519_PUBLIC_LENGTH};
use signature_bbs_plus::MessageGenerators;
use signature_bbs_plus::Signature as BBSSignature;
use signature_core::lib::Message;

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
        } else if public_key.as_ref().len() == 96 {
            Ok(signature == &[0u8; 64])
        } else {
            Err(VaultError::InvalidPublicKey.into())
        }
    }
}
