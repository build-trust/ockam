use crate::{constants, KeyId, Secret, SecretAttributes, SecretType, StoredSecret};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Result};
use ockam_node::{FileValueStorage, InMemoryKeyValueStorage, KeyValueStorage, ValueStorage};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::Path;

/// Storage for a Vault data backed by a file
/// The `FileValueStorage` implementation takes care of locking / unlocking the underlying file
/// in the presence of concurrent accesses
pub struct PersistentStorage {
    storage: Arc<FileValueStorage<StoredSecrets>>,
    cache: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl PersistentStorage {
    /// Create a new file storage for a Vault
    pub async fn create(path: &Path) -> Result<Arc<dyn KeyValueStorage<KeyId, StoredSecret>>> {
        let storage = Arc::new(FileValueStorage::create(path).await?);
        let cache = InMemoryKeyValueStorage::create();
        Ok(Arc::new(PersistentStorage { storage, cache }))
    }
}

/// This struct is serialized to a file in order to persist vault data
#[derive(Debug, Clone, Default)]
struct StoredSecrets {
    secrets: BTreeMap<KeyId, StoredSecret>,
}

#[derive(Serialize, Deserialize)]
struct FileSecrets(Vec<FileSecret>);

#[derive(Serialize, Deserialize)]
struct FileSecret {
    key_id: KeyId,
    secret: Secret,
    attributes: SecretAttributes,
}

impl StoredSecrets {
    fn add_stored_secret(&mut self, key_id: KeyId, stored_secret: StoredSecret) {
        self.secrets.insert(key_id, stored_secret);
    }

    fn get_stored_secret(&self, key_id: &KeyId) -> Option<StoredSecret> {
        self.secrets.get(key_id).cloned()
    }

    fn delete_stored_secret(&mut self, key_id: &KeyId) -> Option<StoredSecret> {
        self.secrets.remove(key_id)
    }
}

impl Serialize for StoredSecrets {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut file_secrets = vec![];
        for (key_id, secret) in self.secrets.iter() {
            file_secrets.push(FileSecret {
                key_id: key_id.clone(),
                secret: secret.secret().clone(),
                attributes: secret.attributes(),
            });
        }
        FileSecrets(file_secrets).serialize(serializer)
    }
}

/// The deserialization for StoredSecrets needs to account for some changes in the stored data
///   - AWS keys are not stored anymore
///   - the persistence field for secrets is not necessary anymore
///   - the length of a secret is only needed for some secret types
impl<'de> Deserialize<'de> for StoredSecrets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum OptionalSecret {
            Key(Secret),
            Aws(KeyId),
        }

        #[derive(Deserialize)]
        enum SecretPersistence {
            Ephemeral,
            Persistent,
        }

        #[derive(Deserialize)]
        struct SecretAttributesV1 {
            stype: SecretType,
            #[allow(dead_code)]
            persistence: SecretPersistence,
            length: u32,
        }

        #[derive(Deserialize)]
        struct LegacyVaultEntry {
            key_id: Option<String>,
            key_attributes: SecretAttributesV1,
            key: OptionalSecret,
        }

        #[derive(Deserialize)]
        #[serde(tag = "version")]
        #[non_exhaustive]
        enum StoredSecretsV1 {
            V1 {
                entries: Vec<(usize, LegacyVaultEntry)>,
                next_id: usize,
            },
        }

        #[derive(Deserialize)]
        struct StoredSecretsV2(FileSecrets);

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Secrets {
            V1(StoredSecretsV1),
            V2(StoredSecretsV2),
        }
        match Secrets::deserialize(deserializer) {
            Ok(Secrets::V1(StoredSecretsV1::V1 {
                entries,
                next_id: _next_id,
            })) => {
                let mut secrets: BTreeMap<KeyId, StoredSecret> = Default::default();
                for (_, entry) in entries {
                    if let (Some(key_id), OptionalSecret::Key(s)) = (entry.key_id, entry.key) {
                        // in principle there should be no elements with no key ids
                        // and we can skip aws key ids which we don't need to store

                        let attributes = match entry.key_attributes.stype {
                            SecretType::Buffer => {
                                SecretAttributes::Buffer(entry.key_attributes.length)
                            }
                            SecretType::Aes => {
                                if entry.key_attributes.length
                                    == constants::AES128_SECRET_LENGTH_U32
                                {
                                    SecretAttributes::Aes128
                                } else {
                                    SecretAttributes::Aes256
                                }
                            }
                            SecretType::X25519 => SecretAttributes::X25519,
                            SecretType::Ed25519 => SecretAttributes::Ed25519,
                            SecretType::NistP256 => SecretAttributes::NistP256,
                        };
                        secrets.insert(key_id, StoredSecret::new(s, attributes));
                    };
                }
                Ok(StoredSecrets { secrets })
            }
            Ok(Secrets::V2(StoredSecretsV2(file_secrets))) => {
                let mut secrets: BTreeMap<KeyId, StoredSecret> = Default::default();
                for secret in file_secrets.0 {
                    secrets.insert(
                        secret.key_id,
                        StoredSecret::new(secret.secret, secret.attributes),
                    );
                }
                Ok(StoredSecrets { secrets })
            }
            Err(e) => Err(e),
        }
    }
}

/// A PersistentStorage can be seen as a key / value store
/// where we read the full data structure and put or get the wanted secret
#[async_trait]
impl KeyValueStorage<KeyId, StoredSecret> for PersistentStorage {
    async fn put(&self, key_id: KeyId, stored_secret: StoredSecret) -> Result<()> {
        self.cache
            .put(key_id.clone(), stored_secret.clone())
            .await?;

        let t = move |mut v: StoredSecrets| {
            v.add_stored_secret(key_id.clone(), stored_secret.clone());
            Ok(v)
        };
        self.storage.update_value(t).await
    }

    async fn get(&self, key_id: &KeyId) -> Result<Option<StoredSecret>> {
        if let Ok(Some(s)) = self.cache.get(key_id).await {
            return Ok(Some(s));
        }
        let k = key_id.clone();
        let t =
            move |v: StoredSecrets| -> Result<Option<StoredSecret>> { Ok(v.get_stored_secret(&k)) };
        self.storage.read_value(t).await
    }

    async fn delete(&self, key_id: &KeyId) -> Result<Option<StoredSecret>> {
        self.cache.delete(key_id).await?;
        let k = key_id.clone();
        let t = move |mut v: StoredSecrets| -> Result<(StoredSecrets, Option<StoredSecret>)> {
            let r = v.delete_stored_secret(&k);
            Ok((v, r))
        };
        self.storage.modify_value(t).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{Secret, SecretType, VaultSecurityModule};
    use ockam_core::compat::rand::RngCore;
    use rand::thread_rng;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_persistent_storage() -> Result<()> {
        let temp_file = create_temp_file();
        let storage = PersistentStorage::create(&temp_file).await?;

        // create and retrieve a persistent secret
        let secret = Secret::new(vec![1; 32]);
        let attributes = SecretAttributes::Ed25519;
        let key_id = VaultSecurityModule::compute_key_id(&secret, &attributes).await?;
        let stored_secret = StoredSecret::new(secret.clone(), attributes);
        storage.put(key_id.clone(), stored_secret.clone()).await?;

        let mut file = File::open(temp_file).expect("Unable to open file");
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)
            .expect("Unable to read file");
        let expected = r#"[{"key_id":"34750f98bd59fcfc946da45aaabe933be154a4b5094e1c4abf42866505f3c97e","secret":"0101010101010101010101010101010101010101010101010101010101010101","attributes":"Ed25519"}]"#;
        assert_eq!(file_contents, expected);

        let missing_key_id: KeyId = "missing-key-id".into();
        let actual = storage.get(&missing_key_id).await?;
        assert_eq!(actual, None);

        let actual = storage.get(&key_id).await?;
        assert_eq!(actual, Some(stored_secret));
        Ok(())
    }

    /// This test check that it is still possible to read the legacy vault file format
    /// AWS key ids are skipped since we don't need to persist them
    #[test]
    fn test_parse_legacy_file_storage() {
        let json = r#"{
           "version":"V1",
           "entries":[
             [0,{"key_id":"12356789","key_attributes":{"stype":"Ed25519","persistence":"Persistent","length":32},"key":{"Key":"010203"}}],
             [1,{"key_id":"aws-key","key_attributes":{"stype":"Ed25519","persistence":"Persistent","length":32},"key":{"Aws":"aws-key"}}]
           ],
           "next_id":0
        }"#;

        let stored_secrets: StoredSecrets = serde_json::from_str(json).unwrap();

        assert_eq!(1, stored_secrets.secrets.len());
        let (key_id, stored_secret) = stored_secrets.secrets.first_key_value().unwrap();

        assert_eq!(key_id, "12356789");
        assert_eq!(
            stored_secret.attributes().secret_type(),
            SecretType::Ed25519
        );
        assert_eq!(stored_secret.attributes().length(), 32);
        let secret_key = Secret::new(vec![1, 2, 3]);
        assert_eq!(stored_secret.secret(), &secret_key);
    }

    pub fn create_temp_file() -> PathBuf {
        let dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let file_name = hex::encode(bytes);
        dir.join(file_name)
    }
}
