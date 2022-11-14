use cfg_if::cfg_if;
use crate::vault::Vault;
use crate::VaultError;
use ockam_core::vault::{KeyId, SecretType, Signature, Signer};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(feature = "ring")]
use crate::error::{from_ring_unspecified, ring_key_rejected};

#[cfg(feature = "rustcrypto")]
use crate::error::from_pkcs8;

#[async_trait]
impl Signer for Vault {
    /// Sign data.
    async fn sign(&self, secret_key: &KeyId, data: &[u8]) -> Result<Signature> {
        self.preload_from_storage(secret_key).await;

        let entries = self.data.entries.read().await;
        let entry = entries.get(secret_key).ok_or(VaultError::EntryNotFound)?;

        let key = entry.key().as_ref();
        match entry.key_attributes().stype() {
            SecretType::X25519 => {
                use crate::xeddsa::XEddsaSigner;
                use arrayref::array_ref;
                use ockam_core::compat::rand::{thread_rng, RngCore};
                use ockam_core::vault::CURVE25519_SECRET_LENGTH_USIZE;
                if key.len() != CURVE25519_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidX25519SecretLength.into());
                }

                let mut rng = thread_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig = x25519_dalek::StaticSecret::from(*array_ref!(
                    key,
                    0,
                    CURVE25519_SECRET_LENGTH_USIZE
                ))
                .xeddsa_sign(data.as_ref(), &nonce);
                Ok(Signature::new(sig.to_vec()))
            }
            SecretType::Ed25519 => {
                use ed25519_dalek::Signer;
                let sk = ed25519_dalek::SecretKey::from_bytes(key).unwrap();
                let pk = ed25519_dalek::PublicKey::from(&sk);

                let kp = ed25519_dalek::Keypair {
                    public: pk,
                    secret: sk,
                };

                let sig = kp.sign(data.as_ref());
                Ok(Signature::new(sig.to_bytes().to_vec()))
            }
            SecretType::NistP256 => {
                cfg_if! {
                    if #[cfg(feature = "ring")] {
                        use ring::rand::SystemRandom;
                        use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair};
                        let sec = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, key)
                            .map_err(ring_key_rejected)?;
                        let rng = SystemRandom::new();
                        let sig = sec.sign(&rng, data).map_err(from_ring_unspecified)?;
                        Ok(Signature::new(sig.as_ref().to_vec()))
                    } else if #[cfg(feature = "rustcrypto")] {
                        use p256::ecdsa::{self, signature::Signer as _};
                        use p256::pkcs8::DecodePrivateKey;
                        let sec = ecdsa::SigningKey::from_pkcs8_der(key).map_err(from_pkcs8)?;
                        let sig = sec.sign(data);
                        Ok(Signature::new(sig.to_der().as_bytes().to_vec()))
                    } else {
                        Err(VaultError::InvalidKeyType.into())
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn sign() {}
}
