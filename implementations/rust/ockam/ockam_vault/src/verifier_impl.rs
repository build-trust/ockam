use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaVerifier;
use crate::VaultError;
use arrayref::array_ref;
use ockam_vault_core::{PublicKey, Verifier, CURVE25519_PUBLIC_LENGTH};

impl Verifier for SoftwareVault {
    /// Verify signature with xeddsa algorithm. Only curve25519 is supported.
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &PublicKey,
        data: &[u8],
    ) -> ockam_core::Result<bool> {
        // FIXME
        if public_key.as_ref().len() == CURVE25519_PUBLIC_LENGTH {
            Ok(x25519_dalek::PublicKey::from(*array_ref!(
                public_key.as_ref(),
                0,
                CURVE25519_PUBLIC_LENGTH
            ))
            .verify(data.as_ref(), signature))
        } else {
            Err(VaultError::InvalidPublicKey.into())
        }
    }
}
