use crate::vault::Vault;
use crate::VaultError;
use ockam_core::vault::{Secret, SecretType, Signature, Signer};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

#[async_trait]
impl Signer for Vault {
    /// Sign data with xeddsa algorithm. Only curve25519 is supported.
    async fn sign(&self, secret_key: &Secret, data: &[u8]) -> Result<Signature> {
        let entries = self.entries.read().await;
        let entry = entries
            .get(&secret_key.index())
            .ok_or(VaultError::EntryNotFound)?;

        let key = entry.key().as_ref();
        match entry.key_attributes().stype() {
            SecretType::X25519 => {
                use crate::xeddsa::XEddsaSigner;
                use arrayref::array_ref;
                use ockam_core::compat::rand::{thread_rng, RngCore};
                use ockam_core::vault::CURVE25519_SECRET_LENGTH;
                if key.len() != CURVE25519_SECRET_LENGTH {
                    return Err(VaultError::InvalidX25519SecretLength.into());
                }

                let mut rng = thread_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig =
                    x25519_dalek::StaticSecret::from(*array_ref!(key, 0, CURVE25519_SECRET_LENGTH))
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
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                use arrayref::array_ref;
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
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
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
