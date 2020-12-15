use crate::error::*;
use ockam_vault::types::{PublicKey, SecretAttributes, SecretKey, SecretPersistence};
use ockam_vault::{
    AsymmetricVault, HashVault, PersistentVault, Secret, SecretVault, SignerVault, SymmetricVault,
    VerifierVault,
};
use ockam_vault_software::DefaultVault;
use std::cmp::max;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

use ockam_common::error::OckamResult;
pub use ockam_vault;

pub mod error;

const ATTRS_BYTE_LENGTH: usize = 6;

/// A FilesystemVault is an implementation of an Ockam Vault that wraps the software vault and uses
/// the disk as a persistent store.
#[derive(Debug)]
pub struct FilesystemVault {
    v: DefaultVault,
    path: PathBuf,
    map: BTreeMap<usize, Box<dyn Secret>>,
    next_id: usize,
}

pub const FILENAME_KEY_SUFFIX: &str = ".key";

/// Default vault secret
#[derive(Debug, Copy, Clone)]
pub struct FilesystemVaultSecret(usize);

impl FilesystemVaultSecret {
    pub fn downcast_secret(context: &Box<dyn Secret>) -> OckamResult<&Self> {
        context
            .downcast_ref::<FilesystemVaultSecret>()
            .map_err(|_| Error::SecretFromAnotherVault.into())
    }
}

impl Zeroize for FilesystemVaultSecret {
    fn zeroize(&mut self) {}
}

impl Secret for FilesystemVaultSecret {}

impl FilesystemVault {
    fn get_entry_map<'a>(
        map: &'a BTreeMap<usize, Box<dyn Secret>>,
        context: &'a Box<dyn Secret>,
    ) -> OckamResult<&'a Box<dyn Secret>> {
        let context = FilesystemVaultSecret::downcast_secret(context)?;
        map.get(&context.0).ok_or(Error::InvalidSecret.into())
    }

    /// Creates a new FilesystemVault using the provided path on disk to store secrets.
    pub fn new(path: PathBuf) -> OckamResult<Self> {
        let mut map = BTreeMap::<usize, Box<dyn Secret>>::new();
        let mut next_id: usize = 0;

        let create_path = path.clone();
        fs::create_dir_all(create_path).or_else(|_| Err(Error::IOError.into()))?;

        let mut vault = DefaultVault::default();
        let to_secret = |data: &[u8]| -> OckamResult<(SecretKey, SecretAttributes)> {
            if data.len() < ATTRS_BYTE_LENGTH {
                return Err(Error::InvalidSecret.into());
            }

            let mut attrs = [0u8; ATTRS_BYTE_LENGTH];
            attrs.copy_from_slice(&data[0..ATTRS_BYTE_LENGTH]);
            let attributes = SecretAttributes::try_from(attrs)?;

            Ok((
                SecretKey::new(data[ATTRS_BYTE_LENGTH..].to_vec()),
                attributes,
            ))
        };
        let fs_path = path.clone();

        path.read_dir()
            .or_else(|_| Err(Error::IOError.into()))?
            .filter(|r| {
                // ignore directories within vault path
                if let Ok(e) = r {
                    match fs::metadata(e.path()) {
                        Ok(md) => md.is_file(),
                        Err(_) => false,
                    }
                } else {
                    false
                }
            })
            .for_each(|entry| {
                if let Ok(entry) = entry {
                    match fs::read(entry.path()) {
                        Ok(data) => {
                            let (secret, attrs) =
                                &to_secret(data.as_slice()).unwrap_or_else(|_| {
                                    panic!("failed to get secret {:?} from file", entry.file_name())
                                });
                            // Files are read in any order
                            let fname = entry.file_name();
                            let t: &std::path::Path = fname.as_os_str().as_ref();
                            let mut item_id: Option<usize> = None;
                            if let Some(stem) = t.file_stem() {
                                if let Some(str) = stem.to_str() {
                                    if let Ok(id) = str.parse::<usize>() {
                                        item_id = Some(id);
                                    }
                                }
                            }
                            match item_id {
                                Some(id) => {
                                    let secret = vault.secret_import(secret.as_ref(), *attrs);
                                    match secret {
                                        Ok(secret) => {
                                            map.insert(id, secret);
                                            next_id = max(next_id, id);
                                        }
                                        Err(e) => eprintln!("{}", e),
                                    }
                                }
                                None => eprintln!("invalid key file name: {:?}", entry),
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                }
            });

        Ok(Self {
            v: vault,
            map,
            path: fs_path,
            next_id,
        })
    }

    fn add_secret(&mut self, secret: Box<dyn Secret>) -> usize {
        self.next_id += 1;
        self.map.insert(self.next_id, secret);
        self.next_id
    }
}

fn id_to_path(id: usize) -> PathBuf {
    format!("{}.key", id.to_string()).into()
}

fn fs_write_secret(
    path: PathBuf,
    id: usize,
    key: &[u8],
    attrs: SecretAttributes,
) -> OckamResult<()> {
    if matches!(attrs.persistence, SecretPersistence::Persistent) {
        let mut bytes = attrs.to_bytes().to_vec();
        bytes.extend_from_slice(key.as_ref());

        fs::write(path.join(id_to_path(id)), bytes).or_else(|_| Err(Error::IOError.into()))?;
    }
    return Ok(());
}

impl SecretVault for FilesystemVault {
    /// Create a new secret key
    fn secret_generate(&mut self, attributes: SecretAttributes) -> OckamResult<Box<dyn Secret>> {
        // write the secret to disk using the context id
        let ctx = self.v.secret_generate(attributes)?;
        let secret = self.v.secret_export(&ctx)?;
        let id = self.add_secret(ctx);
        fs_write_secret(self.path.clone(), id, secret.as_ref(), attributes)?;

        Ok(Box::new(FilesystemVaultSecret(id)))
    }

    /// Import a secret key into the vault
    fn secret_import(
        &mut self,
        secret: &[u8],
        attributes: SecretAttributes,
    ) -> OckamResult<Box<dyn Secret>> {
        // write the secret to disk using the context id
        let ctx = self.v.secret_import(secret, attributes)?;
        let id = self.add_secret(ctx);
        fs_write_secret(self.path.clone(), id, &secret, attributes)?;

        Ok(Box::new(FilesystemVaultSecret(id)))
    }

    /// Export a secret key from the vault
    fn secret_export(&mut self, context: &Box<dyn Secret>) -> OckamResult<SecretKey> {
        let context = Self::get_entry_map(&self.map, context)?;
        self.v.secret_export(context)
    }

    /// Get the attributes for a secret key
    fn secret_attributes_get(
        &mut self,
        context: &Box<dyn Secret>,
    ) -> OckamResult<SecretAttributes> {
        let context = Self::get_entry_map(&self.map, context)?;
        self.v.secret_attributes_get(context)
    }

    /// Return the associated public key given the secret key
    fn secret_public_key_get(&mut self, context: &Box<dyn Secret>) -> OckamResult<PublicKey> {
        let context = Self::get_entry_map(&self.map, context)?;
        self.v.secret_public_key_get(context)
    }

    /// Remove a secret key from the vault
    fn secret_destroy(&mut self, context: Box<dyn Secret>) -> OckamResult<()> {
        let id = FilesystemVaultSecret::downcast_secret(&context)?.0;

        let path = self.path.join(id_to_path(id));
        match fs::metadata(path.clone()) {
            Ok(md) if md.is_file() => {
                fs::remove_file(path).map_err(|_| Error::IOError.into())?;
            }
            _ => {}
        }

        let context = FilesystemVaultSecret::downcast_secret(&context)?;
        let context = self
            .map
            .remove(&context.0)
            .ok_or(Error::EntryNotFound.into())?;
        self.v.secret_destroy(context)?;

        Ok(())
    }
}

impl AsymmetricVault for FilesystemVault {
    /// Compute Elliptic-Curve Diffie-Hellman using this secret key
    ///
    /// and the specified uncompressed public key
    fn ec_diffie_hellman(
        &mut self,
        context: &Box<dyn Secret>,
        peer_public_key: &[u8],
    ) -> OckamResult<Box<dyn Secret>> {
        let context = Self::get_entry_map(&self.map, context)?;
        let ecdh = self.v.ec_diffie_hellman(context, peer_public_key)?;
        let id = self.add_secret(ecdh);
        // TODO: What if ecdh result is persistent?

        Ok(Box::new(FilesystemVaultSecret(id)))
    }
}

impl SymmetricVault for FilesystemVault {
    /// Encrypt a payload using AES-GCM
    fn aead_aes_gcm_encrypt(
        &mut self,
        context: &Box<dyn Secret>,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>> {
        let context = Self::get_entry_map(&self.map, context)?;
        self.v.aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
    }

    /// Decrypt a payload using AES-GCM
    fn aead_aes_gcm_decrypt(
        &mut self,
        context: &Box<dyn Secret>,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> OckamResult<Vec<u8>> {
        let context = Self::get_entry_map(&self.map, context)?;
        self.v
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
    }
}

impl SignerVault for FilesystemVault {
    fn sign(&mut self, secret_key: &Box<dyn Secret>, data: &[u8]) -> OckamResult<[u8; 64]> {
        let context = Self::get_entry_map(&self.map, secret_key)?;
        self.v.sign(context, data)
    }
}

impl VerifierVault for FilesystemVault {
    fn verify(&mut self, signature: &[u8; 64], public_key: &[u8], data: &[u8]) -> OckamResult<()> {
        self.v.verify(signature, public_key, data)
    }
}

impl HashVault for FilesystemVault {
    /// Compute the SHA-256 digest given input `data`
    fn sha256(&self, data: &[u8]) -> OckamResult<[u8; 32]> {
        self.v.sha256(data)
    }
    /// Compute the HKDF-SHA256 using the specified salt and input key material
    ///
    /// and return the output key material of the specified length
    fn hkdf_sha256(
        &mut self,
        salt: &Box<dyn Secret>,
        info: &[u8],
        ikm: Option<&Box<dyn Secret>>,
        output_attributes: Vec<SecretAttributes>,
    ) -> OckamResult<Vec<Box<dyn Secret>>> {
        let ikm = match ikm {
            Some(secret) => Some(Self::get_entry_map(&self.map, secret)?),
            None => None,
        };
        let salt_context = Self::get_entry_map(&self.map, salt)?;

        self.v
            .hkdf_sha256(salt_context, info, ikm, output_attributes)?
            .into_iter()
            .map(|secret| {
                let id = self.add_secret(secret);
                // TODO: What if result is persistent?
                // TODO: Should we remove it

                let res: Box<dyn Secret> = Box::new(FilesystemVaultSecret(id));
                Ok(res)
            })
            .collect()
    }
}
impl PersistentVault for FilesystemVault {
    fn get_persistence_id(&self, secret: &Box<dyn Secret>) -> OckamResult<String> {
        let id = FilesystemVaultSecret::downcast_secret(secret)?.0;
        Ok(format!("{}{}", id, FILENAME_KEY_SUFFIX))
    }

    fn get_persistent_secret(&self, persistence_id: &str) -> OckamResult<Box<dyn Secret>> {
        let id = persistence_id
            .strip_suffix(FILENAME_KEY_SUFFIX)
            .ok_or(Error::InvalidPersistenceId.into())?;
        let id: usize = id.parse().map_err(|_| Error::InvalidPersistenceId.into())?;

        if self.map.contains_key(&id) {
            Ok(Box::new(FilesystemVaultSecret(id)))
        } else {
            Err(Error::InvalidPersistenceId.into())
        }
    }
}

impl Zeroize for FilesystemVault {
    fn zeroize(&mut self) {
        self.v.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_vault::types::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};

    #[test]
    fn persistence_test() {
        let path = std::path::PathBuf::from("__persistence_test");
        if path.exists() {
            std::fs::remove_dir_all(path.clone()).unwrap();
        }
        let mut vault = FilesystemVault::new(path.clone()).unwrap();
        let atts = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Persistent,
            length: CURVE25519_SECRET_LENGTH,
        };
        let sk1 = vault.secret_generate(atts).unwrap();
        let sk2 = vault.secret_generate(atts).unwrap();
        let sk3 = vault.secret_generate(atts).unwrap();

        let sk_data1 = vault.secret_export(&sk1).unwrap();
        let sk_data2 = vault.secret_export(&sk2).unwrap();
        let sk_data3 = vault.secret_export(&sk3).unwrap();

        let sk1_persistence_id = vault.get_persistence_id(&sk1).unwrap();
        let sk2_persistence_id = vault.get_persistence_id(&sk2).unwrap();
        let sk3_persistence_id = vault.get_persistence_id(&sk3).unwrap();

        let mut vault2 = FilesystemVault::new(path).unwrap();
        let sk1 = vault2.get_persistent_secret(&sk1_persistence_id).unwrap();
        let sk2 = vault2.get_persistent_secret(&sk2_persistence_id).unwrap();
        let sk3 = vault2.get_persistent_secret(&sk3_persistence_id).unwrap();
        let sk2_data_1 = vault2.secret_export(&sk1).unwrap();
        let sk2_data_2 = vault2.secret_export(&sk2).unwrap();
        let sk2_data_3 = vault2.secret_export(&sk3).unwrap();

        assert_eq!(sk_data1, sk2_data_1);
        assert_eq!(sk_data2, sk2_data_2);
        assert_eq!(sk_data3, sk2_data_3);
    }
}
