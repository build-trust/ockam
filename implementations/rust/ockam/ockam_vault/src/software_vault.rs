use crate::VaultError;
use ockam_vault_core::zdrop_impl;
use ockam_vault_core::{Secret, SecretAttributes, SecretKey};
use std::collections::BTreeMap;
use zeroize::Zeroize;

#[derive(Debug)]
pub struct SoftwareVault {
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    pub(crate) next_id: usize,
}

impl SoftwareVault {
    pub fn new() -> Self {
        Self {
            entries: Default::default(),
            next_id: 0,
        }
    }
}

impl Default for SoftwareVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareVault {
    pub(crate) fn get_entry(&self, context: &Secret) -> Result<&VaultEntry, ockam_core::Error> {
        self.entries
            .get(&context.index())
            .ok_or_else(|| VaultError::EntryNotFound.into())
    }
}

impl Zeroize for SoftwareVault {
    fn zeroize(&mut self) {
        for (_, v) in self.entries.iter_mut() {
            v.zeroize();
        }
        self.entries.clear();
        self.next_id = 0;
    }
}

zdrop_impl!(SoftwareVault);

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VaultEntry {
    kid: Option<String>,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl VaultEntry {
    pub fn kid(&self) -> &Option<String> {
        &self.kid
    }
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }
    pub fn key(&self) -> &SecretKey {
        &self.key
    }
}

impl VaultEntry {
    pub fn new(kid: Option<String>, key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            kid,
            key_attributes,
            key,
        }
    }
}

impl Zeroize for VaultEntry {
    fn zeroize(&mut self) {
        self.key.zeroize()
    }
}

zdrop_impl!(VaultEntry);

#[cfg(test)]
mod tests {
    use crate::software_vault::SoftwareVault;
    use ockam_vault_core::HashVault;
    use ockam_vault_core::SecretVault;
    use ockam_vault_core::SignerVault;
    use ockam_vault_core::VerifierVault;
    use ockam_vault_core::{
        SecretAttributes, SecretPersistence, SecretType, CURVE25519_PUBLIC_LENGTH,
        CURVE25519_SECRET_LENGTH,
    };

    #[test]
    fn new_vault() {
        let vault = SoftwareVault::new();
        assert_eq!(vault.next_id, 0);
        assert_eq!(vault.entries.len(), 0);
    }

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
    fn sha256() {
        let vault = SoftwareVault::default();
        let res = vault.sha256(b"a");
        assert!(res.is_ok());
        let digest = res.unwrap();
        assert_eq!(
            hex::encode(digest),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn hkdf() {
        let mut vault = SoftwareVault::default();

        let salt_value = b"hkdf_test";
        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: salt_value.len(),
        };
        let salt = vault.secret_import(&salt_value[..], attributes).unwrap();

        let ikm_value = b"a";
        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: ikm_value.len(),
        };
        let ikm = vault.secret_import(&ikm_value[..], attributes).unwrap();

        let attributes = SecretAttributes {
            stype: SecretType::Buffer,
            persistence: SecretPersistence::Ephemeral,
            length: 24,
        };

        let res = vault.hkdf_sha256(&salt, b"", Some(&ikm), vec![attributes]);
        assert!(res.is_ok());
        let digest = res.unwrap();
        assert_eq!(digest.len(), 1);
        let digest = vault.secret_export(&digest[0]).unwrap();
        assert_eq!(
            hex::encode(digest.as_ref()),
            "921ab9f260544b71941dbac2ca2d42c417aa07b53e055a8f"
        );
    }

    #[test]
    fn sign() {
        let mut vault = SoftwareVault::default();
        let secret = vault
            .secret_generate(SecretAttributes {
                persistence: SecretPersistence::Ephemeral,
                stype: SecretType::Curve25519,
                length: CURVE25519_SECRET_LENGTH,
            })
            .unwrap();
        let res = vault.sign(&secret, b"hello world!");
        assert!(res.is_ok());
        let pubkey = vault.secret_public_key_get(&secret).unwrap();
        let signature = res.unwrap();
        let res = vault.verify(&signature, pubkey.as_ref(), b"hello world!");
        assert!(res.is_ok());
    }
}
