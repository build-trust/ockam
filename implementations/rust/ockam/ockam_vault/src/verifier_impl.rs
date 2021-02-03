use crate::error::Error;
use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaVerifier;
use arrayref::array_ref;
use ockam_vault_core::{VerifierVault, CURVE25519_PUBLIC_LENGTH};

impl VerifierVault for SoftwareVault {
    fn verify(
        &mut self,
        signature: &[u8; 64],
        public_key: &[u8],
        data: &[u8],
    ) -> Result<(), ockam_core::Error> {
        // FIXME
        if public_key.len() == CURVE25519_PUBLIC_LENGTH {
            if x25519_dalek::PublicKey::from(*array_ref!(public_key, 0, CURVE25519_PUBLIC_LENGTH))
                .verify(data.as_ref(), &signature)
            {
                Ok(())
            } else {
                Err(Error::InvalidSignature.into())
            }
        } else {
            Err(Error::InvalidPublicKey.into())
        }
    }
}
