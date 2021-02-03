use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaSigner;
use crate::VaultError;
use arrayref::array_ref;
use ockam_vault_core::{Secret, SecretType, SignerVault, CURVE25519_SECRET_LENGTH};
use rand::{thread_rng, RngCore};

impl SignerVault for SoftwareVault {
    /// Sign data with xeddsa algorithm. Only curve25519 is supported.
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> ockam_core::Result<[u8; 64]> {
        let entry = self.get_entry(secret_key)?;
        let key = entry.key().as_ref();
        match entry.key_attributes().stype {
            SecretType::Curve25519 if key.len() == CURVE25519_SECRET_LENGTH => {
                let mut rng = thread_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig =
                    x25519_dalek::StaticSecret::from(*array_ref!(key, 0, CURVE25519_SECRET_LENGTH))
                        .sign(data.as_ref(), &nonce);
                Ok(sig)
            }
            _ => Err(VaultError::InvalidKeyType.into()),
        }
    }
}
