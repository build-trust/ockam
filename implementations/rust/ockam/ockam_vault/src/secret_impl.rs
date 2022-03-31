use crate::vault::{Vault, VaultEntry};
use crate::VaultError;
use arrayref::array_ref;
use cfg_if::cfg_if;
use core::sync::atomic::Ordering;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::vault::{
    KeyId, KeyIdVault, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence,
    SecretType, SecretVault, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH, CURVE25519_SECRET_LENGTH,
};
use ockam_core::{async_trait, compat::boxed::Box, Result};
cfg_if! {
    if #[cfg(feature = "bls")] {
        use signature_bbs_plus::PublicKey as BlsPublicKey;
        use signature_bbs_plus::SecretKey as BlsSecretKey;
    }
}

impl Vault {
    /// Compute key id from secret and attributes. Only Curve25519 and Buffer types are supported
    async fn compute_key_id(
        &self,
        secret: &[u8],
        attributes: &SecretAttributes,
    ) -> Result<Option<KeyId>> {
        Ok(match attributes.stype() {
            SecretType::X25519 => {
                // FIXME: Check secret length
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    secret,
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let public = x25519_dalek::PublicKey::from(&sk);
                Some(
                    self.compute_key_id_for_public_key(&PublicKey::new(
                        public.as_bytes().to_vec(),
                        SecretType::X25519,
                    ))
                    .await?,
                )
            }
            SecretType::Ed25519 => {
                let sk = ed25519_dalek::SecretKey::from_bytes(secret)
                    .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let public = ed25519_dalek::PublicKey::from(&sk);
                Some(
                    self.compute_key_id_for_public_key(&PublicKey::new(
                        public.as_bytes().to_vec(),
                        SecretType::Ed25519,
                    ))
                    .await?,
                )
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                let bls_secret_key = BlsSecretKey::from_bytes(secret.try_into().unwrap()).unwrap();
                let public_key = PublicKey::new(
                    BlsPublicKey::from(&bls_secret_key).to_bytes().into(),
                    SecretType::Bls,
                );
                Some(self.compute_key_id_for_public_key(&public_key).await?)
            }
            SecretType::Buffer | SecretType::Aes => None,
        })
    }

    /// Validate secret key.
    pub fn check_secret(&self, secret: &[u8], attributes: &SecretAttributes) -> Result<()> {
        if secret.len() != attributes.length() {
            return Err(VaultError::InvalidSecretLength.into());
        }
        match attributes.stype() {
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                let bytes = TryInto::<[u8; BlsSecretKey::BYTES]>::try_into(secret)
                    .map_err(|_| VaultError::InvalidBlsSecretLength)?;
                if BlsSecretKey::from_bytes(&bytes).is_none().into() {
                    return Err(VaultError::InvalidBlsSecret.into());
                }
            }
            SecretType::Buffer | SecretType::Aes | SecretType::X25519 | SecretType::Ed25519 => {
                // Avoid unused variable warning
                let _ = secret;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl SecretVault for Vault {
    /// Generate fresh secret. Only Curve25519 and Buffer types are supported
    async fn secret_generate(&self, attributes: SecretAttributes) -> Result<Secret> {
        let key = match attributes.stype() {
            SecretType::X25519 | SecretType::Ed25519 => {
                let bytes = {
                    let mut rng = thread_rng();
                    let mut bytes = vec![0u8; CURVE25519_SECRET_LENGTH];
                    rng.fill_bytes(&mut bytes);
                    bytes
                };

                SecretKey::new(bytes)
            }
            SecretType::Buffer => {
                if attributes.persistence() != SecretPersistence::Ephemeral {
                    return Err(VaultError::InvalidKeyType.into());
                };
                let key = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length()];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };

                SecretKey::new(key)
            }
            SecretType::Aes => {
                if attributes.length() != AES256_SECRET_LENGTH
                    && attributes.length() != AES128_SECRET_LENGTH
                {
                    return Err(VaultError::InvalidAesKeyLength.into());
                };
                if attributes.persistence() != SecretPersistence::Ephemeral {
                    return Err(VaultError::InvalidKeyType.into());
                };
                let key = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length()];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };

                SecretKey::new(key)
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                let mut rng = thread_rng();
                let bls_secret_key = BlsSecretKey::random(&mut rng).unwrap();

                SecretKey::new(bls_secret_key.to_bytes().to_vec())
            }
        };
        let key_id = self.compute_key_id(key.as_ref(), &attributes).await?;

        let next_id = self.data.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.data
            .entries
            .write()
            .await
            .insert(next_id, VaultEntry::new(key_id, attributes, key));

        Ok(Secret::new(next_id))
    }

    #[tracing::instrument(skip_all, err)]
    async fn secret_import(&self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
        self.check_secret(secret, &attributes)?;
        let key_id_opt = self.compute_key_id(secret, &attributes).await?;
        let next_id = self.data.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.data.entries.write().await.insert(
            next_id,
            VaultEntry::new(key_id_opt, attributes, SecretKey::new(secret.to_vec())),
        );
        Ok(Secret::new(next_id))
    }

    async fn secret_export(&self, context: &Secret) -> Result<SecretKey> {
        Ok(self
            .data
            .entries
            .read()
            .await
            .get(&context.index())
            .ok_or(VaultError::EntryNotFound)?
            .key()
            .clone())
    }

    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes> {
        Ok(self
            .data
            .entries
            .read()
            .await
            .get(&context.index())
            .ok_or(VaultError::EntryNotFound)?
            .key_attributes())
    }

    /// Extract public key from secret. Only Curve25519 type is supported
    async fn secret_public_key_get(&self, context: &Secret) -> Result<PublicKey> {
        let entries = self.data.entries.read().await;
        let entry = entries
            .get(&context.index())
            .ok_or(VaultError::EntryNotFound)?;

        match entry.key_attributes().stype() {
            SecretType::X25519 => {
                if entry.key().as_ref().len() != CURVE25519_SECRET_LENGTH {
                    return Err(VaultError::InvalidPrivateKeyLen.into());
                }

                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.key().as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::X25519))
            }
            SecretType::Ed25519 => {
                if entry.key().as_ref().len() != CURVE25519_SECRET_LENGTH {
                    return Err(VaultError::InvalidPrivateKeyLen.into());
                }

                let sk = ed25519_dalek::SecretKey::from_bytes(entry.key().as_ref())
                    .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let pk = ed25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::Ed25519))
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => {
                let bls_secret_key =
                    BlsSecretKey::from_bytes(&entry.key().as_ref().try_into().unwrap()).unwrap();
                Ok(PublicKey::new(
                    BlsPublicKey::from(&bls_secret_key).to_bytes().into(),
                    SecretType::Bls,
                ))
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
        }
    }

    /// Remove secret from memory
    async fn secret_destroy(&self, context: Secret) -> Result<()> {
        match self.data.entries.write().await.remove(&context.index()) {
            None => Err(VaultError::EntryNotFound.into()),
            Some(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ockam_core::vault::{KeyId, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH},
        KeyIdVault, Secret, SecretAttributes, SecretVault, Vault,
    };
    use cfg_if::cfg_if;

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
            CURVE25519_SECRET_LENGTH,
        ))
    }

    fn new_ed255519_attrs() -> Option<SecretAttributes> {
        Some(SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        ))
    }

    fn new_bls_attrs() -> Option<SecretAttributes> {
        cfg_if! {
            if #[cfg(feature = "bls")] {
                use signature_bbs_plus::SecretKey as BlsSecretKey;
                Some(SecretAttributes::new(
                    SecretType::Bls,
                    SecretPersistence::Ephemeral,
                    BlsSecretKey::BYTES,
                ))
            }
            else {
                None
            }
        }
    }

    async fn check_key_id_computation(vault: Vault, sec_idx: Secret) {
        let public_key = vault.secret_public_key_get(&sec_idx).await.unwrap();
        let key_id = vault
            .compute_key_id_for_public_key(&public_key)
            .await
            .unwrap();
        let sec_idx_2 = vault.secret_by_key_id(&key_id).await.unwrap();
        assert_eq!(sec_idx, sec_idx_2)
    }

    fn flat_map_options<T>(vec: Vec<Option<T>>) -> Vec<T> {
        vec.into_iter()
            .flat_map(|x| match x {
                None => vec![],
                Some(y) => vec![y],
            })
            .collect()
    }

    #[tokio::test]
    async fn secret_generate_compute_key_id() {
        for attrs in flat_map_options(vec![
            new_x255519_attrs(),
            new_ed255519_attrs(),
            new_bls_attrs(),
        ]) {
            let vault = new_vault();
            let sec_idx = vault.secret_generate(attrs).await.unwrap();
            check_key_id_computation(vault, sec_idx).await;
        }
    }

    #[tokio::test]
    async fn secret_import_compute_key_id() {
        for attrs in flat_map_options(vec![
            new_x255519_attrs(),
            new_ed255519_attrs(),
            new_bls_attrs(),
        ]) {
            let vault = new_vault();
            let sec_idx = vault.secret_generate(attrs).await.unwrap();
            let secret = vault.secret_export(&sec_idx).await.unwrap();
            drop(vault); // The first vault was only used to generate random keys

            let vault = new_vault();
            let sec_idx = vault.secret_import(secret.as_ref(), attrs).await.unwrap();

            check_key_id_computation(vault, sec_idx).await;
        }
    }

    async fn import_key(vault: &mut Vault, bytes: &[u8], attrs: SecretAttributes) -> KeyId {
        let sec_idx = vault.secret_import(bytes, attrs).await.unwrap();
        let public_key = vault.secret_public_key_get(&sec_idx).await.unwrap();
        vault
            .compute_key_id_for_public_key(&public_key)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn secret_import_compute_key_id_predefined() {
        let bytes_c25519 = &[
            0x48, 0x95, 0x73, 0xcf, 0x4a, 0xe9, 0x16, 0x68, 0x86, 0x49, 0x8d, 0x3d, 0xd0, 0xde,
            0x00, 0x61, 0xb4, 0x01, 0xc1, 0xbf, 0x39, 0xd0, 0x8b, 0x7e, 0x4b, 0xf0, 0xa4, 0x90,
            0xbb, 0x1c, 0x91, 0x67,
        ];
        let attrs = new_x255519_attrs().unwrap();
        let mut vault = new_vault();
        let key_id = import_key(&mut vault, bytes_c25519, attrs).await;
        assert_eq!(
            "f0e6821043434a9353e6c213a098f6d75ac916b23b3632c7c4c9c6d2e1fa1cf8",
            &key_id
        );

        cfg_if! {
            if #[cfg(feature = "bls")] {
                let bytes_bls = &[
                    0x3b, 0xcd, 0x36, 0xf3, 0xe2, 0x18, 0xf1, 0x8a, 0x37, 0xd6, 0x4d, 0x62, 0xe4, 0xb7,
                    0x27, 0xc9, 0xc7, 0xf8, 0xcc, 0x32, 0x6c, 0xf6, 0x66, 0x94, 0x7c, 0x62, 0xfd, 0xff,
                    0x18, 0xc0, 0x0e, 0x08,
                ];
                let attrs = new_bls_attrs().unwrap();
                let mut vault = new_vault();
                let key_id = import_key(&mut vault, bytes_bls, attrs).await;
                assert_eq!(
                    "604b7cf225a832c8fa822792dc7c484f5c49fb7a70ce87f1636b294ba7dbdc7b",
                    &key_id
                );
            }
        }
    }
}
