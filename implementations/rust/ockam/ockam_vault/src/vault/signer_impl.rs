use crate::{KeyId, SecretType, Signature, Signer, Vault, VaultError};
use cfg_if::cfg_if;
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(feature = "rustcrypto")]
use crate::from_pkcs8;
use crate::storage::SecretStorage;

#[async_trait]
impl Signer for Vault {
    /// Sign data.
    async fn sign(&self, secret_key: &KeyId, data: &[u8]) -> Result<Signature> {
        let vault_entry = self.get_secret(secret_key, "signing key").await?;

        match vault_entry.key_attributes().stype() {
            SecretType::X25519 => {
                use crate::vault::xeddsa::XEddsaSigner;
                use crate::CURVE25519_SECRET_LENGTH_USIZE;
                use arrayref::array_ref;
                use ockam_core::compat::rand::{thread_rng, RngCore};
                let key = vault_entry.secret().try_as_key()?.as_ref();
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
                let key = vault_entry.secret().try_as_key()?.as_ref();
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
                let key = vault_entry.secret().try_as_key()?.as_ref();
                cfg_if! {
                    if #[cfg(feature = "rustcrypto")] {
                        use p256::ecdsa::signature::Signer;
                        use p256::pkcs8::DecodePrivateKey;
                        let sec = p256::ecdsa::SigningKey::from_pkcs8_der(key).map_err(from_pkcs8)?;

                        let sig: p256::ecdsa::Signature = sec.sign(data);
                        Ok(Signature::new(sig.to_der().as_bytes().to_vec()))
                    } else {
                        compile_error!("NIST P-256 requires feature `rustcrypto`")
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn sign() {}
}
