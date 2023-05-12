use crate::{KeyId, PublicKey, Secret, SecretAttributes, StoredSecret};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// This traits provides all the functionalities related to the management of secrets
#[async_trait]
pub trait SecretsStore:
    EphemeralSecretsStore + PersistentSecretsStore + SecretsStoreReader + Sync + Send
{
}

impl<T: EphemeralSecretsStore + PersistentSecretsStore + SecretsStoreReader + Sync + Send>
    SecretsStore for T
{
}

/// This traits supports the creation / retrieval / deletion of ephemeral secrets
#[async_trait]
pub trait EphemeralSecretsStore: SecretsStoreReader + Sync + Send {
    /// Generate a secret and persist it to ephemeral memory
    async fn create_ephemeral_secret(&self, attributes: SecretAttributes) -> Result<KeyId>;
    /// Import a secret and persist it to ephemeral memory
    async fn import_ephemeral_secret(
        &self,
        secret: Secret,
        attributes: SecretAttributes,
    ) -> Result<KeyId>;
    /// Export an ephemeral secret
    /// Use the description in an error message if the secret cannot be found
    async fn get_ephemeral_secret(&self, key_id: &KeyId, description: &str)
        -> Result<StoredSecret>;
    /// Remove an ephemeral secret from the vault.
    async fn delete_ephemeral_secret(&self, key_id: KeyId) -> Result<bool>;
}

/// This traits supports the creation / deletion of persistent secrets
#[async_trait]
pub trait PersistentSecretsStore: SecretsStoreReader + Sync + Send {
    /// Generate a secret and persist it to long-term memory
    async fn create_persistent_secret(&self, attributes: SecretAttributes) -> Result<KeyId>;
    /// Remove a persistent secret from the vault.
    async fn delete_persistent_secret(&self, key_id: KeyId) -> Result<bool>;
}

/// This traits supports the retrieval of public information for a given secret
#[async_trait]
pub trait SecretsStoreReader: Sync + Send {
    /// Return the secret attributes related to a secret
    async fn get_secret_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes>;
    /// Return the associated public key given the secret key
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;
    /// Compute and return the `KeyId` for a given public key.
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId>;
}

/// Tests for implementations of the SecretsStore trait
#[cfg(feature = "vault_tests")]
pub mod tests {
    use super::*;
    use crate::{PublicKey, SecretAttributes, SecretType};
    use hex::decode;
    use ockam_core::compat::vec::Vec;
    use SecretType::*;

    /// This test checks the creation of ephemeral keys of different types
    pub async fn test_create_ephemeral_secrets(vault: &mut impl SecretsStore) {
        for attributes in all_secret_attributes() {
            let key_id = vault.create_ephemeral_secret(attributes).await.unwrap();

            // once a secret is created we can retrieve it  using the key id
            let secret = vault.get_ephemeral_secret(&key_id, "secret").await.unwrap();
            assert_eq!(secret.attributes(), attributes);

            // once a secret is created we can get its public key using the key id
            // for secrets that are not Buffer or Aes secrets
            let public_key = vault.get_public_key(&key_id).await;
            let secret_type = attributes.secret_type();
            if ![Buffer, Aes].contains(&secret_type) {
                // the public key must have a suitable length
                assert!(public_key.unwrap().data().len() >= 32);
            } else {
                assert!(public_key.is_err())
            }

            // finally we can delete the secret using its key id
            let deleted = vault.delete_ephemeral_secret(key_id).await.unwrap();
            assert!(deleted);
        }
    }

    /// This test checks that we can import and export ephemeral secrets
    pub async fn test_secret_import_export(vault: &mut impl SecretsStore) {
        for attributes in all_secret_attributes() {
            let key_id = vault.create_ephemeral_secret(attributes).await.unwrap();
            let secret = vault.get_ephemeral_secret(&key_id, "secret").await.unwrap();

            let new_key_id = vault
                .import_ephemeral_secret(secret.secret().clone(), attributes)
                .await
                .unwrap();

            assert_eq!(
                vault
                    .get_ephemeral_secret(&new_key_id, "secret")
                    .await
                    .unwrap(),
                secret
            );
        }
    }

    /// This tests checks that we can retrieve attributes from both ephemeral and persistent secrets
    pub async fn test_get_secret_attributes(vault: &mut impl SecretsStore) {
        for attributes in all_secret_attributes() {
            let secret = vault.create_ephemeral_secret(attributes).await.unwrap();
            assert_eq!(
                vault.get_secret_attributes(&secret).await.unwrap(),
                attributes
            );

            let secret = vault.create_persistent_secret(attributes).await.unwrap();
            assert_eq!(
                vault.get_secret_attributes(&secret).await.unwrap(),
                attributes
            );
        }
    }

    /// This tests checks that we can compute a key id from a public key
    pub async fn test_get_key_id_by_public_key(vault: &mut impl SecretsStore) {
        for attributes in [
            SecretAttributes::Ed25519,
            SecretAttributes::X25519,
            SecretAttributes::NistP256,
        ] {
            let public =
                decode("68858ea1ea4e1ade755df7fb6904056b291d9781eb5489932f46e32f12dd192a").unwrap();
            let public = PublicKey::new(public.to_vec(), attributes.secret_type());
            let key_id = vault.get_key_id(&public).await.unwrap();

            assert_eq!(
                key_id,
                "732af49a0b47c820c0a4cac428d6cb80c1fa70622f4a51708163dd87931bc942"
            );
        }
    }

    /// This test checks that we can create a persistent secret then retrieve its key id from
    /// its public key
    pub async fn test_get_key_id_for_persistent_secret_from_public_key(
        vault: &mut impl SecretsStore,
    ) {
        for attributes in [
            SecretAttributes::Ed25519,
            SecretAttributes::X25519,
            SecretAttributes::NistP256,
        ] {
            let secret = vault.create_persistent_secret(attributes).await.unwrap();
            let public = vault.get_public_key(&secret).await.unwrap();
            let key_id = vault.get_key_id(&public).await.unwrap();
            assert_eq!(secret, key_id);
        }
    }

    /// Return all the types of secret attributes
    fn all_secret_attributes() -> Vec<SecretAttributes> {
        vec![
            SecretAttributes::Ed25519,
            SecretAttributes::X25519,
            SecretAttributes::Buffer(32),
            SecretAttributes::Aes128,
            SecretAttributes::Aes256,
            SecretAttributes::NistP256,
        ]
    }
}
