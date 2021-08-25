use crate::software_vault::{SoftwareVault, VaultEntry};
use crate::VaultError;
use arrayref::array_ref;
use core::convert::TryInto;
use ockam_vault_core::{
    KeyId, KeyIdVault, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence,
    SecretType, SecretVault, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH, CURVE25519_SECRET_LENGTH,
};
use signature_bbs_plus::PublicKey as BlsPublicKey;
use signature_bbs_plus::SecretKey as BlsSecretKey;
use zeroize::Zeroize;

#[cfg(not(feature = "std"))]
use ockam_core::compat::rand::{thread_rng, RngCore};
#[cfg(feature = "std")]
use rand::{thread_rng, RngCore};

impl SoftwareVault {
    /// Compute key id from secret and attributes. Only Curve25519 and Buffer types are supported
    fn compute_key_id(
        &mut self,
        secret: &[u8],
        attributes: &SecretAttributes,
    ) -> ockam_core::Result<Option<KeyId>> {
        Ok(match attributes.stype() {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    secret,
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let public = x25519_dalek::PublicKey::from(&sk);
                Some(
                    self.compute_key_id_for_public_key(&PublicKey::new(
                        public.as_bytes().to_vec(),
                    ))?,
                )
            }
            SecretType::Bls => {
                let bls_secret_key = BlsSecretKey::from_bytes(secret.try_into().unwrap()).unwrap();
                let public_key =
                    PublicKey::new(BlsPublicKey::from(&bls_secret_key).to_bytes().into());
                Some(self.compute_key_id_for_public_key(&public_key)?)
            }
            SecretType::Buffer | SecretType::Aes | SecretType::P256 => None,
        })
    }

    /// Validate secret key.
    pub fn check_secret(
        &mut self,
        secret: &[u8],
        attributes: &SecretAttributes,
    ) -> ockam_core::Result<()> {
        match attributes.stype() {
            SecretType::Bls => {
                let bytes = TryInto::<[u8; BlsSecretKey::BYTES]>::try_into(secret)
                    .map_err(|_| VaultError::InvalidBlsSecretLength)?;
                if BlsSecretKey::from_bytes(&bytes).is_none().into() {
                    return Err(VaultError::InvalidBlsSecret.into());
                }
            }
            SecretType::Buffer | SecretType::Aes | SecretType::Curve25519 => {}
            SecretType::P256 => { /* FIXME */ }
        }
        Ok(())
    }
}

impl SecretVault for SoftwareVault {
    /// Generate fresh secret. Only Curve25519 and Buffer types are supported
    fn secret_generate(&mut self, attributes: SecretAttributes) -> ockam_core::Result<Secret> {
        let mut rng = thread_rng(); // FIXME
        let key = match attributes.stype() {
            SecretType::Curve25519 => {
                let mut bytes = vec![0u8; 32];
                rng.fill_bytes(&mut bytes);

                SecretKey::new(bytes)
            }
            SecretType::Buffer => {
                if attributes.persistence() != SecretPersistence::Ephemeral {
                    return Err(VaultError::InvalidKeyType.into());
                };
                let mut key = vec![0u8; attributes.length()];
                rng.fill_bytes(key.as_mut_slice());
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
                let mut key = vec![0u8; attributes.length()];
                rng.fill_bytes(&mut key);
                SecretKey::new(key)
            }
            SecretType::P256 => {
                return Err(VaultError::InvalidKeyType.into());
            }
            SecretType::Bls => {
                let bls_secret_key = BlsSecretKey::random(&mut rng).unwrap();

                SecretKey::new(bls_secret_key.to_bytes().to_vec())
            }
        };
        let key_id = self.compute_key_id(key.as_ref(), &attributes)?;
        self.next_id += 1;
        self.entries
            .insert(self.next_id, VaultEntry::new(key_id, attributes, key));

        Ok(Secret::new(self.next_id))
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> ockam_core::Result<Secret> {
        self.check_secret(secret, &attributes)?;
        let key_id_opt = self.compute_key_id(secret, &attributes)?;
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry::new(key_id_opt, attributes, SecretKey::new(secret.to_vec())),
        );
        Ok(Secret::new(self.next_id))
    }

    fn secret_export(&mut self, context: &Secret) -> ockam_core::Result<SecretKey> {
        self.get_entry(context).map(|i| i.key().clone())
    }

    fn secret_attributes_get(&mut self, context: &Secret) -> ockam_core::Result<SecretAttributes> {
        self.get_entry(context).map(|i| i.key_attributes())
    }

    /// Extract public key from secret. Only Curve25519 type is supported
    fn secret_public_key_get(&mut self, context: &Secret) -> ockam_core::Result<PublicKey> {
        let entry = self.get_entry(context)?;

        if entry.key().as_ref().len() != CURVE25519_SECRET_LENGTH {
            return Err(VaultError::InvalidPrivateKeyLen.into());
        }

        match entry.key_attributes().stype() {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.key().as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec()))
            }
            SecretType::Bls => {
                let bls_secret_key =
                    BlsSecretKey::from_bytes(&entry.key().as_ref().try_into().unwrap()).unwrap();
                Ok(PublicKey::new(
                    BlsPublicKey::from(&bls_secret_key).to_bytes().into(),
                ))
            }
            SecretType::Buffer | SecretType::Aes | SecretType::P256 => {
                Err(VaultError::InvalidKeyType.into())
            }
        }
    }

    /// Remove secret from memory
    fn secret_destroy(&mut self, context: Secret) -> ockam_core::Result<()> {
        if let Some(mut k) = self.entries.remove(&context.index()) {
            k.zeroize();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ockam_vault_core::{KeyId, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH},
        KeyIdVault, Secret, SecretAttributes, SecretVault, SoftwareVault,
    };
    use ockam_vault_test_attribute::*;
    use signature_bbs_plus::SecretKey as BlsSecretKey;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test]
    fn new_public_keys() {}

    #[vault_test]
    fn new_secret_keys() {}

    #[vault_test]
    fn secret_import_export() {}

    #[vault_test]
    fn secret_attributes_get() {}

    fn new_curve255519_attrs() -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        )
    }

    fn new_bls_attrs() -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Bls,
            SecretPersistence::Ephemeral,
            BlsSecretKey::BYTES,
        )
    }

    fn check_key_id_computation(mut vault: SoftwareVault, sec_idx: Secret) {
        let public_key = vault.secret_public_key_get(&sec_idx).unwrap();
        let key_id = vault.compute_key_id_for_public_key(&public_key).unwrap();
        let sec_idx_2 = vault.get_secret_by_key_id(&key_id).unwrap();
        assert_eq!(sec_idx, sec_idx_2)
    }

    #[test]
    fn secret_generate_compute_key_id() {
        for attrs in &[new_curve255519_attrs(), new_bls_attrs()] {
            let mut vault = new_vault();
            let sec_idx = vault.secret_generate(*attrs).unwrap();
            check_key_id_computation(vault, sec_idx);
        }
    }

    #[test]
    fn secret_import_compute_key_id() {
        for attrs in &[new_curve255519_attrs(), new_bls_attrs()] {
            let mut vault = new_vault();
            let sec_idx = vault.secret_generate(*attrs).unwrap();
            let secret = vault.secret_export(&sec_idx).unwrap();
            drop(vault); // The first vault was only used to generate random keys

            let mut vault = new_vault();
            let sec_idx = vault.secret_import(secret.as_ref(), *attrs).unwrap();

            check_key_id_computation(vault, sec_idx);
        }
    }

    fn import_key(vault: &mut SoftwareVault, bytes: &[u8], attrs: SecretAttributes) -> KeyId {
        let sec_idx = vault.secret_import(bytes, attrs).unwrap();
        let public_key = vault.secret_public_key_get(&sec_idx).unwrap();
        vault.compute_key_id_for_public_key(&public_key).unwrap()
    }

    #[test]
    fn secret_import_compute_key_id_predefined() {
        let bytes_c25519 = &[
            0x48, 0x95, 0x73, 0xcf, 0x4a, 0xe9, 0x16, 0x68, 0x86, 0x49, 0x8d, 0x3d, 0xd0, 0xde,
            0x00, 0x61, 0xb4, 0x01, 0xc1, 0xbf, 0x39, 0xd0, 0x8b, 0x7e, 0x4b, 0xf0, 0xa4, 0x90,
            0xbb, 0x1c, 0x91, 0x67,
        ];
        let attrs = new_curve255519_attrs();
        let mut vault = new_vault();
        let key_id = import_key(&mut vault, bytes_c25519, attrs);
        assert_eq!(
            "f0e6821043434a9353e6c213a098f6d75ac916b23b3632c7c4c9c6d2e1fa1cf8",
            &key_id
        );

        let bytes_bls = &[
            0x3b, 0xcd, 0x36, 0xf3, 0xe2, 0x18, 0xf1, 0x8a, 0x37, 0xd6, 0x4d, 0x62, 0xe4, 0xb7,
            0x27, 0xc9, 0xc7, 0xf8, 0xcc, 0x32, 0x6c, 0xf6, 0x66, 0x94, 0x7c, 0x62, 0xfd, 0xff,
            0x18, 0xc0, 0x0e, 0x08,
        ];
        let attrs = new_bls_attrs();
        let mut vault = new_vault();
        let key_id = import_key(&mut vault, bytes_bls, attrs);
        assert_eq!(
            "604b7cf225a832c8fa822792dc7c484f5c49fb7a70ce87f1636b294ba7dbdc7b",
            &key_id
        );
    }
}
