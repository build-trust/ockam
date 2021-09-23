use crate::software_vault::SoftwareVault;
use crate::xeddsa::XEddsaSigner;
use crate::VaultError;
use arrayref::array_ref;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_vault_core::{Secret, SecretType, Signature, Signer, CURVE25519_SECRET_LENGTH};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl Signer for SoftwareVault {
    /// Sign data with xeddsa algorithm. Only curve25519 is supported.
    fn sign(&mut self, secret_key: &Secret, data: &[u8]) -> ockam_core::Result<Signature> {
        let entry = self.get_entry(secret_key)?;
        let key = entry.key().as_ref();
        match entry.key_attributes().stype() {
            SecretType::Curve25519 => {
                if key.len() == CURVE25519_SECRET_LENGTH {
                    let mut rng = thread_rng();
                    let mut nonce = [0u8; 64];
                    rng.fill_bytes(&mut nonce);
                    let sig = x25519_dalek::StaticSecret::from(*array_ref!(
                        key,
                        0,
                        CURVE25519_SECRET_LENGTH
                    ))
                    .sign(data.as_ref(), &nonce);
                    Ok(Signature::new(sig.to_vec()))
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                use signature_bbs_plus::{Issuer, MessageGenerators};
                use signature_bls::SecretKey;
                use signature_core::lib::Message;
                if key.len() == 32 {
                    let bls_secret_key = SecretKey::from_bytes(array_ref!(key, 0, 32)).unwrap();
                    let generators = MessageGenerators::from_secret_key(&bls_secret_key, 1);
                    let messages = [Message::hash(data)];
                    let sig = Issuer::sign(&bls_secret_key, &generators, &messages).unwrap();
                    Ok(Signature::new(sig.to_bytes().to_vec()))
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            SecretType::Buffer | SecretType::Aes | SecretType::P256 => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }

    /// Sign data with xeddsa algorithm. Only curve25519 is supported.
    async fn async_sign(
        &mut self,
        secret_key: &Secret,
        data: &[u8],
    ) -> ockam_core::Result<Signature> {
        self.sign(secret_key, data)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test]
    fn sign() {}
}
