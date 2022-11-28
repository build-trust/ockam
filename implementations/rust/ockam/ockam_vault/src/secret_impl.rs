use crate::vault::Vault;
use crate::VaultError;
use arrayref::array_ref;
use cfg_if::cfg_if;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::vault::{
    AsymmetricVault, KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence,
    SecretType, SecretVault, VaultEntry, AES128_SECRET_LENGTH_U32, AES256_SECRET_LENGTH_U32,
    CURVE25519_SECRET_LENGTH_USIZE,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};

#[cfg(any(feature = "evercrypt", feature = "rustcrypto"))]
use crate::error::{from_ecurve, from_pkcs8};

impl Vault {
    /// Compute key id from secret and attributes. Only Curve25519 and Buffer types are supported
    async fn compute_key_id(
        &self,
        secret: &Secret,
        attributes: &SecretAttributes,
    ) -> Result<KeyId> {
        Ok(match attributes.stype() {
            SecretType::X25519 => {
                // FIXME: Check secret length
                let secret = secret.try_as_key()?.as_ref();
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    secret,
                    0,
                    CURVE25519_SECRET_LENGTH_USIZE
                ]);
                let public = x25519_dalek::PublicKey::from(&sk);

                self.compute_key_id_for_public_key(&PublicKey::new(
                    public.as_bytes().to_vec(),
                    SecretType::X25519,
                ))
                .await?
            }
            SecretType::Ed25519 => {
                let sk = ed25519_dalek::SecretKey::from_bytes(secret.try_as_key()?.as_ref())
                    .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let public = ed25519_dalek::PublicKey::from(&sk);

                self.compute_key_id_for_public_key(&PublicKey::new(
                    public.as_bytes().to_vec(),
                    SecretType::Ed25519,
                ))
                .await?
            }
            SecretType::NistP256 => '_block: {
                #[cfg(feature = "aws")]
                if attributes.persistence() == SecretPersistence::Persistent {
                    if let Some(kms) = &self.aws_kms {
                        if let Secret::Aws(kid) = secret {
                            let pk = kms.public_key(kid).await?;
                            break '_block self.compute_key_id_for_public_key(&pk).await?;
                        }
                    }
                }
                cfg_if! {
                    if #[cfg(any(feature = "evercrypt", feature = "rustcrypto"))] {
                        let pk = public_key(secret.try_as_key()?.as_ref())?;
                        self.compute_key_id_for_public_key(&pk).await?
                    } else {
                        return Err(VaultError::InvalidKeyType.into())
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => {
                // NOTE: Buffer and Aes secrets in the system are ephemeral and it should be fine,
                // that every time we import the same secret - it gets different KeyId value.
                // However, if we decide to have persistent Buffer or Aes secrets, that should be
                // change (probably to hash value of the secret)
                let mut rng = thread_rng();
                let mut rand = [0u8; 8];
                rng.fill_bytes(&mut rand);
                hex::encode(rand)
            }
        })
    }

    /// Validate secret key.
    pub fn check_secret(&self, secret: &[u8], attributes: &SecretAttributes) -> Result<()> {
        if secret.len() != attributes.length() as usize {
            return Err(VaultError::InvalidSecretLength.into());
        }
        Ok(())
    }

    async fn store_secret(&self, key_id: &KeyId, vault_entry: &VaultEntry) -> Result<()> {
        if vault_entry.key_attributes().persistence() == SecretPersistence::Persistent {
            if let Some(storage) = &self.storage {
                storage.store(key_id, vault_entry).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SecretVault for Vault {
    /// Generate fresh secret. Only Curve25519 and Buffer types are supported
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let secret = match attributes.stype() {
            SecretType::X25519 | SecretType::Ed25519 => {
                let bytes = {
                    let mut rng = thread_rng();
                    let mut bytes = vec![0u8; CURVE25519_SECRET_LENGTH_USIZE];
                    rng.fill_bytes(&mut bytes);
                    bytes
                };

                Secret::Key(SecretKey::new(bytes))
            }
            SecretType::Buffer => {
                if attributes.persistence() != SecretPersistence::Ephemeral {
                    return Err(VaultError::InvalidKeyType.into());
                };
                let key = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length() as usize];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };

                Secret::Key(SecretKey::new(key))
            }
            SecretType::Aes => {
                if attributes.length() != AES256_SECRET_LENGTH_U32
                    && attributes.length() != AES128_SECRET_LENGTH_U32
                {
                    return Err(VaultError::InvalidAesKeyLength.into());
                };
                if attributes.persistence() != SecretPersistence::Ephemeral {
                    return Err(VaultError::InvalidKeyType.into());
                };
                let key = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length() as usize];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };

                Secret::Key(SecretKey::new(key))
            }
            SecretType::NistP256 => '_block: {
                #[cfg(feature = "aws")]
                if attributes.persistence() == SecretPersistence::Persistent {
                    if let Some(kms) = &self.aws_kms {
                        let aws_id = kms.create_key().await?;
                        break '_block Secret::Aws(aws_id);
                    }
                }
                cfg_if! {
                    if #[cfg(any(feature = "evercrypt", feature = "rustcrypto"))] {
                        use p256::ecdsa::SigningKey;
                        use p256::pkcs8::EncodePrivateKey;
                        let sec = SigningKey::random(thread_rng());
                        let sec = p256::SecretKey::from_be_bytes(&sec.to_bytes()).map_err(from_ecurve)?;
                        let doc = sec.to_pkcs8_der().map_err(from_pkcs8)?;
                        Secret::Key(SecretKey::new(doc.as_bytes().to_vec()))
                    } else {
                        compile_error!("one of features {evercrypt,rustcrypto} must be given")
                    }
                }
            }
        };
        let key_id = self.compute_key_id(&secret, &attributes).await?;

        let entry = VaultEntry::new(attributes, secret);
        self.store_secret(&key_id, &entry).await?;

        self.data
            .entries
            .write()
            .await
            .insert(key_id.clone(), entry);

        Ok(key_id)
    }

    #[tracing::instrument(skip_all, err)]
    async fn secret_import(&self, secret: Secret, attributes: SecretAttributes) -> Result<KeyId> {
        if let Secret::Key(sk) = &secret {
            self.check_secret(sk.as_ref(), &attributes)?
        }
        let key_id = self.compute_key_id(&secret, &attributes).await?;
        let entry = VaultEntry::new(attributes, secret);
        self.store_secret(&key_id, &entry).await?;

        self.data
            .entries
            .write()
            .await
            .insert(key_id.clone(), entry);

        Ok(key_id)
    }

    async fn secret_export(&self, key_id: &KeyId) -> Result<Secret> {
        self.preload_from_storage(key_id).await;
        let entries = self.data.entries.read().await;

        if let Some(entry) = entries.get(key_id) {
            return Ok(entry.secret().clone());
        }

        Err(VaultError::EntryNotFound.into())
    }

    async fn secret_attributes_get(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        self.preload_from_storage(key_id).await;
        let entries = self.data.entries.read().await;

        if let Some(e) = entries.get(key_id) {
            return Ok(e.key_attributes());
        }

        Err(VaultError::EntryNotFound.into())
    }

    /// Extract public key from secret. Only Curve25519 type is supported
    async fn secret_public_key_get(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.preload_from_storage(key_id).await;

        let entries = self.data.entries.read().await;
        let entry = entries.get(key_id).ok_or(VaultError::EntryNotFound)?;

        match entry.key_attributes().stype() {
            SecretType::X25519 => {
                if entry.secret().try_as_key()?.as_ref().len() != CURVE25519_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidPrivateKeyLen.into());
                }

                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.secret().try_as_key()?.as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH_USIZE
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::X25519))
            }
            SecretType::Ed25519 => {
                if entry.secret().try_as_key()?.as_ref().len() != CURVE25519_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidPrivateKeyLen.into());
                }

                let sk =
                    ed25519_dalek::SecretKey::from_bytes(entry.secret().try_as_key()?.as_ref())
                        .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let pk = ed25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::Ed25519))
            }
            SecretType::NistP256 => {
                #[cfg(feature = "aws")]
                if let Some(kms) = &self.aws_kms {
                    if let Secret::Aws(kid) = entry.secret() {
                        return kms.public_key(kid).await;
                    }
                }
                cfg_if! {
                    if #[cfg(any(feature = "evercrypt", feature = "rustcrypto"))] {
                        if let Secret::Key(sk) = entry.secret() {
                            public_key(sk.as_ref())
                        } else {
                            Err(VaultError::InvalidKeyType.into())
                        }
                    } else {
                        Err(VaultError::InvalidKeyType.into())
                    }
                }
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
        }
    }

    /// Remove secret from memory
    async fn secret_destroy(&self, key_id: KeyId) -> Result<()> {
        let attrs = self.secret_attributes_get(&key_id).await?;

        // Acquire lock to avoid race conditions
        let mut entries = self.data.entries.write().await;

        let res = if attrs.persistence() == SecretPersistence::Persistent {
            if let Some(storage) = &self.storage {
                storage.delete(&key_id).await.map(|_| ())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        };

        match entries.remove(&key_id) {
            None => return Err(VaultError::EntryNotFound.into()),
            Some(_entry) =>
            {
                #[cfg(feature = "aws")]
                if let Some(kms) = &self.aws_kms {
                    if let Secret::Aws(kid) = _entry.secret() {
                        if !kms.delete_key(kid).await? {
                            return Err(VaultError::EntryNotFound.into());
                        }
                    }
                }
            }
        }

        res
    }
}

#[cfg(any(feature = "evercrypt", feature = "rustcrypto"))]
fn public_key(secret: &[u8]) -> Result<PublicKey> {
    use p256::pkcs8::{DecodePrivateKey, EncodePublicKey};
    let sec = p256::ecdsa::SigningKey::from_pkcs8_der(secret).map_err(from_pkcs8)?;
    let pky = sec
        .verifying_key()
        .to_public_key_der()
        .map_err(from_pkcs8)?;
    Ok(PublicKey::new(pky.as_ref().to_vec(), SecretType::NistP256))
}

#[cfg(test)]
mod tests {
    use ockam_core::vault::{Secret, SecretKey};

    use crate::{
        ockam_core::vault::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH_U32},
        SecretAttributes, SecretVault, Vault,
    };

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn new_public_keys() {}

    #[ockam_macros::vault_test]
    fn new_secret_keys() {}

    #[ockam_macros::vault_test]
    fn secret_import_export() {}

    #[ockam_macros::vault_test]
    fn secret_attributes_get() {}

    fn new_x255519_attrs() -> Option<SecretAttributes> {
        Some(SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        ))
    }

    #[tokio::test]
    async fn secret_import_compute_key_id_predefined() {
        let bytes_c25519 = vec![
            0x48, 0x95, 0x73, 0xcf, 0x4a, 0xe9, 0x16, 0x68, 0x86, 0x49, 0x8d, 0x3d, 0xd0, 0xde,
            0x00, 0x61, 0xb4, 0x01, 0xc1, 0xbf, 0x39, 0xd0, 0x8b, 0x7e, 0x4b, 0xf0, 0xa4, 0x90,
            0xbb, 0x1c, 0x91, 0x67,
        ];
        let attrs = new_x255519_attrs().unwrap();
        let vault = new_vault();
        let key_id = vault
            .secret_import(Secret::Key(SecretKey::new(bytes_c25519)), attrs)
            .await
            .unwrap();
        assert_eq!(
            "f0e6821043434a9353e6c213a098f6d75ac916b23b3632c7c4c9c6d2e1fa1cf8",
            &key_id
        );
    }
}
