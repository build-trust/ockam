use crate::{KeyId, PublicKey, SecretAttributes, SmallBuffer};
use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Defines the Vault interface for asymmetric encryption.
#[async_trait]
pub trait AsymmetricVault: Send + Sync {
    /// Compute and store an Elliptic-Curve Diffie-Hellman key using a secret key
    /// and an uncompressed public key.
    async fn ec_diffie_hellman(&self, secret: &KeyId, peer_public_key: &PublicKey)
        -> Result<KeyId>;

    /// Derive multiple output [`super::Secret`]s with given attributes using
    /// the HKDF-SHA256 given the specified salt, info and input key
    /// material.
    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<KeyId>>;
}

/// Tests for implementations of the AsymmetricVault trait
#[cfg(feature = "vault_tests")]
pub mod tests {
    use super::*;
    use crate::{EphemeralSecretsStore, Secret, SecretAttributes};
    use hex::encode;

    /// This test checks that we can create a Diffie-Hellman key from 2 public keys
    pub async fn test_ec_diffie_hellman_curve25519(
        vault: &mut (impl AsymmetricVault + EphemeralSecretsStore),
    ) {
        let attributes = SecretAttributes::X25519;
        let key_id_1 = vault.create_ephemeral_secret(attributes).await.unwrap();
        let key_id_2 = vault.create_ephemeral_secret(attributes).await.unwrap();
        // in general this public key is sent by a peer
        let public_key_2 = vault.get_public_key(&key_id_2).await.unwrap();

        // for now we just check that the generation doesn't fail
        // TODO: Check result against test vector
        let secret_key_id = vault
            .ec_diffie_hellman(&key_id_1, &public_key_2)
            .await
            .unwrap();

        let secret = vault
            .get_ephemeral_secret(&secret_key_id, "ecdh_secret")
            .await;
        assert!(secret.is_ok());
    }

    /// This test checks the creation of a derived HKDF key
    pub async fn test_hkdf_sha256(vault: &mut (impl AsymmetricVault + EphemeralSecretsStore)) {
        let salt_value = b"hkdf_test";
        let secret = Secret::new(salt_value.to_vec());
        let attributes = SecretAttributes::Buffer(salt_value.len() as u32);
        let salt = vault
            .import_ephemeral_secret(secret, attributes)
            .await
            .unwrap();

        let ikm_value = b"a";
        let secret = Secret::new(ikm_value.to_vec());
        let attributes = SecretAttributes::Buffer(ikm_value.len() as u32);
        let ikm = vault
            .import_ephemeral_secret(secret, attributes)
            .await
            .unwrap();

        let attributes = SecretAttributes::Buffer(24u32);
        let digest = vault
            .hkdf_sha256(&salt, b"", Some(&ikm), vec![attributes])
            .await
            .unwrap();
        assert_eq!(digest.len(), 1);

        let digest = vault
            .get_ephemeral_secret(&digest[0], "digest")
            .await
            .unwrap();
        assert_eq!(
            encode(digest.secret().as_ref()),
            "921ab9f260544b71941dbac2ca2d42c417aa07b53e055a8f"
        );
    }
}
