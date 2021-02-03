use crate::software_vault::{SoftwareVault, VaultEntry};
use crate::VaultError;
use arrayref::array_ref;
use ockam_vault_core::{
    HashVault, PublicKey, Secret, SecretAttributes, SecretKey, SecretType, SecretVault,
    CURVE25519_SECRET_LENGTH,
};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroize;

impl SecretVault for SoftwareVault {
    /// Generate fresh secret. Only Curve25519 and Buffer types are supported
    fn secret_generate(&mut self, attributes: SecretAttributes) -> ockam_core::Result<Secret> {
        let mut rng = OsRng {};
        let (key, kid) = match attributes.stype {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::new(&mut rng);
                let public = x25519_dalek::PublicKey::from(&sk);
                let private = SecretKey::new(sk.to_bytes().to_vec());
                let kid = self.sha256(public.as_bytes())?;

                // FIXME: kid computation should be in one place
                (private, Some(hex::encode(kid)))
            }
            SecretType::Buffer => {
                let mut key = vec![0u8; attributes.length];
                rng.fill_bytes(key.as_mut_slice());
                (SecretKey::new(key), None)
            }
            _ => unimplemented!(),
        };
        self.next_id += 1;
        self.entries
            .insert(self.next_id, VaultEntry::new(kid, attributes, key));

        Ok(Secret::new(self.next_id))
    }

    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> ockam_core::Result<Secret> {
        // FIXME: Should we check secrets here?
        self.next_id += 1;
        self.entries.insert(
            self.next_id,
            VaultEntry::new(
                /* FIXME */ None,
                attributes,
                SecretKey::new(secret.to_vec()),
            ),
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

        match entry.key_attributes().stype {
            SecretType::Curve25519 => {
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    entry.key().as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec()))
            }
            _ => Err(VaultError::InvalidKeyType.into()),
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
    use crate::SoftwareVault;
    use ockam_vault_core::{
        SecretAttributes, SecretPersistence, SecretType, SecretVault, CURVE25519_PUBLIC_LENGTH,
        CURVE25519_SECRET_LENGTH,
    };

    #[test]
    fn new_public_keys() {
        let mut vault = SoftwareVault::default();
        let mut attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let p256_ctx_1 = res.unwrap();

        let res = vault.secret_public_key_get(&p256_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert_eq!(pk_1.as_ref().len(), CURVE25519_PUBLIC_LENGTH);
        assert_eq!(vault.entries.len(), 1);
        assert_eq!(vault.next_id, 1);

        attributes.stype = SecretType::Curve25519;

        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let c25519_ctx_1 = res.unwrap();
        let res = vault.secret_public_key_get(&c25519_ctx_1);
        assert!(res.is_ok());
        let pk_1 = res.unwrap();
        assert_eq!(pk_1.as_ref().len(), CURVE25519_PUBLIC_LENGTH);
        assert_eq!(vault.entries.len(), 2);
        assert_eq!(vault.next_id, 2);
    }

    #[test]
    fn new_secret_keys() {
        let mut vault = SoftwareVault::default();
        let mut attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };
        let types = [(SecretType::Curve25519, 32), (SecretType::Buffer, 24)];
        for (t, s) in &types {
            attributes.stype = *t;
            attributes.length = *s;
            let res = vault.secret_generate(attributes);
            assert!(res.is_ok());
            let sk_ctx = res.unwrap();
            let sk = vault.secret_export(&sk_ctx).unwrap();
            assert_eq!(sk.as_ref().len(), *s);
            vault.secret_destroy(sk_ctx).unwrap();
            assert_eq!(vault.entries.len(), 0);
        }
    }

    #[test]
    fn secret_import_export() {
        let mut vault = SoftwareVault::default();
        let attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };

        let secret_str = "98d589b0dce92c9e2442b3093718138940bff71323f20b9d158218b89c3cec6e";

        let secret = vault
            .secret_import(hex::decode(secret_str).unwrap().as_slice(), attributes)
            .unwrap();

        assert_eq!(secret.index(), 1);
        assert_eq!(
            hex::encode(vault.secret_export(&secret).unwrap().as_ref()),
            secret_str
        );

        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: 24,
        };
        let secret_str = "5f791cc52297f62c7b8829b15f828acbdb3c613371d21aa1";
        let secret = vault
            .secret_import(hex::decode(secret_str).unwrap().as_slice(), attributes)
            .unwrap();

        assert_eq!(secret.index(), 2);

        assert_eq!(
            hex::encode(vault.secret_export(&secret).unwrap().as_ref()),
            secret_str
        );
    }

    #[test]
    fn secret_attributes_get() {
        let mut vault = SoftwareVault::default();

        let attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Ephemeral,
            length: CURVE25519_SECRET_LENGTH,
        };

        let secret = vault.secret_generate(attributes).unwrap();
        assert_eq!(vault.secret_attributes_get(&secret).unwrap(), attributes);

        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: 24,
        };

        let secret = vault.secret_generate(attributes).unwrap();
        assert_eq!(vault.secret_attributes_get(&secret).unwrap(), attributes);
    }
}
