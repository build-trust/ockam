use crate::vault::Vault;
use crate::VaultError;
use cfg_if::cfg_if;
use ockam_core::vault::{KeyId, KeyType, Signature, Signer};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(feature = "aws")]
use ockam_core::vault::Key;

#[cfg(feature = "rustcrypto")]
use crate::error::from_pkcs8;

#[async_trait]
impl Signer for Vault {
    /// Sign data.
    async fn sign(&self, secret_key: &KeyId, data: &[u8]) -> Result<Signature> {
        self.preload_from_storage(secret_key).await;

        let entries = self.data.entries.read().await;
        let entry = entries.get(secret_key).ok_or(VaultError::EntryNotFound)?;

        match entry.key_attributes().stype() {
            KeyType::X25519 => {
                use crate::xeddsa::XEddsaSigner;
                use arrayref::array_ref;
                use ockam_core::compat::rand::{thread_rng, RngCore};
                use ockam_core::vault::CURVE25519_SECRET_LENGTH_USIZE;
                let key = entry.key().try_as_key()?.as_ref();
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
            KeyType::Ed25519 => {
                use ed25519_dalek::Signer;
                let key = entry.key().try_as_key()?.as_ref();
                let sk = ed25519_dalek::SecretKey::from_bytes(key).unwrap();
                let pk = ed25519_dalek::PublicKey::from(&sk);

                let kp = ed25519_dalek::Keypair {
                    public: pk,
                    secret: sk,
                };

                let sig = kp.sign(data.as_ref());
                Ok(Signature::new(sig.to_bytes().to_vec()))
            }
            KeyType::NistP256 => {
                #[cfg(feature = "aws")]
                if let Some(kms) = &self.aws_kms {
                    if let Key::Aws(kid) = entry.key() {
                        return kms.sign(kid, data).await;
                    }
                }
                let key = entry.key().try_as_key()?.as_ref();
                cfg_if! {
                    if #[cfg(feature = "rustcrypto")] {
                        use p256::ecdsa::{self, signature::Signer as _};
                        use p256::pkcs8::DecodePrivateKey;
                        let sec = ecdsa::SigningKey::from_pkcs8_der(key).map_err(from_pkcs8)?;

                        let sig: ecdsa::Signature = sec.sign(data);
                        Ok(Signature::new(sig.to_der().as_bytes().to_vec()))
                    } else {
                        compile_error!("NIST P-256 requires feature `rustcrypto`")
                    }
                }
            }
            KeyType::Buffer | KeyType::Aes => Err(VaultError::InvalidKeyType.into()),
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
