use crate::constants::CURVE25519_PUBLIC_LENGTH_USIZE;
use crate::constants::CURVE25519_SECRET_LENGTH_U32;

use crate::{
    Kms, PublicKey, Secret, SecretAttributes, SecretType, Signature, StoredSecret, VaultError,
};
use arrayref::array_ref;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::compat::sync::Arc;
#[cfg(feature = "rustcrypto")]
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_core::{Error, KeyId};
use ockam_node::{InMemoryKeyValueStorage, KeyValueStorage};
use sha2::{Digest, Sha256};

/// Ockam implementation of a KMS
/// An alternative implementation can be found in the ockam_vault_aws crate
pub struct VaultKms {
    storage: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
}

impl VaultKms {
    /// Create a new Kms
    pub fn create() -> Arc<dyn Kms> {
        Self::create_with_storage(InMemoryKeyValueStorage::create())
    }

    /// Create a new Kms backed by a specific key value storage
    pub fn create_with_storage(
        storage: Arc<dyn KeyValueStorage<KeyId, StoredSecret>>,
    ) -> Arc<dyn Kms> {
        Arc::new(VaultKms { storage })
    }
}

#[async_trait]
impl Kms for VaultKms {
    /// Generate fresh secret
    async fn create_secret(&self, attributes: SecretAttributes) -> Result<KeyId> {
        let secret = Self::create_secret_from_attributes(attributes)?;
        let stored_secret = StoredSecret::create(secret.clone(), attributes)?;
        let key_id = Self::compute_key_id(&secret, &attributes).await?;
        self.storage.put(key_id.clone(), stored_secret).await?;
        Ok(key_id)
    }

    /// Extract public key from secret. Only Curve25519 type is supported
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        let vault_entry = self.get_secret(key_id, "secret public key").await?;
        Self::compute_public_key_from_secret(vault_entry)
    }

    /// Get the key id for a given public key
    async fn get_key_id(&self, public_key: &PublicKey) -> Result<KeyId> {
        let key_id = Self::sha256(public_key.data());
        Ok(hex::encode(key_id))
    }

    /// Get the secret attributes for a given key id
    async fn get_attributes(&self, key_id: &KeyId) -> Result<SecretAttributes> {
        let vault_entry = self.get_secret(key_id, "secret public key").await?;
        Ok(vault_entry.attributes())
    }

    /// Remove secret from memory and persistent storage if it is a persistent secret
    async fn delete_secret(&self, key_id: KeyId) -> Result<bool> {
        self.storage.delete(&key_id).await.map(|r| r.is_some())
    }

    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        match public_key.stype() {
            SecretType::X25519 => {
                if public_key.data().len() != CURVE25519_PUBLIC_LENGTH_USIZE
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }

                use crate::vault::vault_kms::xeddsa::XEddsaVerifier;
                use arrayref::array_ref;

                let signature_array = array_ref!(signature.as_ref(), 0, 64);
                let public_key = x25519_dalek::PublicKey::from(*array_ref!(
                    public_key.data(),
                    0,
                    CURVE25519_PUBLIC_LENGTH_USIZE
                ));
                Ok(public_key.xeddsa_verify(data.as_ref(), signature_array))
            }
            SecretType::Ed25519 => {
                if public_key.data().len() != CURVE25519_PUBLIC_LENGTH_USIZE
                    || signature.as_ref().len() != 64
                {
                    return Err(VaultError::InvalidPublicKey.into());
                }
                use ed25519_dalek::Verifier;

                let signature = ed25519_dalek::Signature::from_bytes(signature.as_ref()).unwrap();
                let public_key = ed25519_dalek::PublicKey::from_bytes(public_key.data()).unwrap();
                Ok(public_key.verify(data.as_ref(), &signature).is_ok())
            }
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => {
                use p256::ecdsa::{signature::Verifier as _, Signature, VerifyingKey};
                use p256::pkcs8::DecodePublicKey;
                let k = VerifyingKey::from_public_key_der(public_key.data())
                    .map_err(Self::from_pkcs8)?;
                let s = Signature::from_der(signature.as_ref()).map_err(Self::from_ecdsa)?;
                Ok(k.verify(data, &s).is_ok())
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidPublicKey.into()),
        }
    }

    async fn sign(&self, key_id: &KeyId, message: &[u8]) -> Result<Signature> {
        let vault_entry = self.get_secret(key_id, "kms signing key").await?;
        Self::sign_with_secret(vault_entry, message)
    }
}

impl VaultKms {
    pub(crate) fn create_secret_from_attributes(attributes: SecretAttributes) -> Result<Secret> {
        let secret = match attributes.secret_type() {
            SecretType::X25519 | SecretType::Ed25519 | SecretType::Buffer | SecretType::Aes => {
                let bytes = {
                    let mut rng = thread_rng();
                    let mut key = vec![0u8; attributes.length() as usize];
                    rng.fill_bytes(key.as_mut_slice());
                    key
                };
                Secret::new(bytes)
            }
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => {
                use p256::ecdsa::SigningKey;
                use p256::pkcs8::EncodePrivateKey;
                let sec = SigningKey::random(&mut thread_rng());
                let sec =
                    p256::SecretKey::from_bytes(&sec.to_bytes()).map_err(Self::from_ecurve)?;
                let doc = sec.to_pkcs8_der().map_err(Self::from_pkcs8)?;
                Secret::new(doc.as_bytes().to_vec())
            }
        };
        Ok(secret)
    }

    pub(crate) fn compute_public_key_from_secret(
        stored_secret: StoredSecret,
    ) -> Result<PublicKey, Error> {
        let attributes = stored_secret.attributes();
        match attributes.secret_type() {
            SecretType::X25519 => {
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    stored_secret.secret().as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH_U32 as usize
                ]);
                let pk = x25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::X25519))
            }
            SecretType::Ed25519 => {
                let sk = ed25519_dalek::SecretKey::from_bytes(stored_secret.secret().as_ref())
                    .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let pk = ed25519_dalek::PublicKey::from(&sk);
                Ok(PublicKey::new(pk.to_bytes().to_vec(), SecretType::Ed25519))
            }
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => Self::public_key(stored_secret.secret().as_ref()),
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
        }
    }

    pub(crate) fn sign_with_secret(
        stored_secret: StoredSecret,
        data: &[u8],
    ) -> Result<Signature, Error> {
        let attributes = stored_secret.attributes();
        match attributes.secret_type() {
            SecretType::X25519 => {
                use crate::vault::vault_kms::xeddsa::XEddsaSigner;
                let key = stored_secret.secret().as_ref();
                let mut rng = thread_rng();
                let mut nonce = [0u8; 64];
                rng.fill_bytes(&mut nonce);
                let sig = x25519_dalek::StaticSecret::from(*array_ref!(
                    key,
                    0,
                    CURVE25519_SECRET_LENGTH_U32 as usize
                ))
                .xeddsa_sign(data.as_ref(), &nonce);
                Ok(Signature::new(sig.to_vec()))
            }
            SecretType::Ed25519 => {
                use ed25519_dalek::Signer;
                let key = stored_secret.secret().as_ref();
                let sk = ed25519_dalek::SecretKey::from_bytes(key).unwrap();
                let pk = ed25519_dalek::PublicKey::from(&sk);

                let kp = ed25519_dalek::Keypair {
                    public: pk,
                    secret: sk,
                };

                let sig = kp.sign(data.as_ref());
                Ok(Signature::new(sig.to_bytes().to_vec()))
            }
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => {
                let key = stored_secret.secret().as_ref();
                use p256::ecdsa::signature::Signer;
                use p256::pkcs8::DecodePrivateKey;
                let sec = p256::ecdsa::SigningKey::from_pkcs8_der(key).map_err(Self::from_pkcs8)?;

                let sig: p256::ecdsa::Signature = sec.sign(data);
                Ok(Signature::new(sig.to_der().as_bytes().to_vec()))
            }
            SecretType::Buffer | SecretType::Aes => Err(VaultError::InvalidKeyType.into()),
        }
    }

    /// Compute key id from secret and attributes
    pub(crate) async fn compute_key_id(
        secret: &Secret,
        attributes: &SecretAttributes,
    ) -> Result<KeyId> {
        Ok(match attributes.secret_type() {
            SecretType::X25519 => {
                let secret = secret.as_ref();
                let sk = x25519_dalek::StaticSecret::from(*array_ref![
                    secret,
                    0,
                    CURVE25519_SECRET_LENGTH_U32 as usize
                ]);
                let public = x25519_dalek::PublicKey::from(&sk);

                Self::compute_key_id_for_public_key(&PublicKey::new(
                    public.as_bytes().to_vec(),
                    SecretType::X25519,
                ))
                .await?
            }
            SecretType::Ed25519 => {
                let sk = ed25519_dalek::SecretKey::from_bytes(secret.as_ref())
                    .map_err(|_| VaultError::InvalidEd25519Secret)?;
                let public = ed25519_dalek::PublicKey::from(&sk);

                Self::compute_key_id_for_public_key(&PublicKey::new(
                    public.as_bytes().to_vec(),
                    SecretType::Ed25519,
                ))
                .await?
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
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => {
                let pk = Self::public_key(secret.as_ref())?;
                Self::compute_key_id_for_public_key(&pk).await?
            }
        })
    }

    pub(crate) async fn compute_key_id_for_public_key(public_key: &PublicKey) -> Result<KeyId> {
        let key_id = Self::sha256(public_key.data());
        Ok(hex::encode(key_id))
    }

    #[cfg(feature = "rustcrypto")]
    fn public_key(secret: &[u8]) -> Result<PublicKey> {
        use p256::pkcs8::{DecodePrivateKey, EncodePublicKey};
        let sec = p256::ecdsa::SigningKey::from_pkcs8_der(secret).map_err(Self::from_pkcs8)?;
        let pky = sec
            .verifying_key()
            .to_public_key_der()
            .map_err(Self::from_pkcs8)?;
        Ok(PublicKey::new(pky.as_ref().to_vec(), SecretType::NistP256))
    }

    /// The sha256 is a constant function which must always refer to the same implementation
    /// wherever it is used
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
    }

    #[cfg(feature = "rustcrypto")]
    pub(crate) fn from_ecdsa(e: p256::ecdsa::Error) -> Error {
        Error::new(Origin::Vault, Kind::Unknown, e)
    }

    #[cfg(feature = "rustcrypto")]
    pub(crate) fn from_pkcs8<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;

        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }

    #[cfg(feature = "rustcrypto")]
    pub(crate) fn from_ecurve(e: p256::elliptic_curve::Error) -> Error {
        Error::new(Origin::Vault, Kind::Unknown, e)
    }
}

impl VaultKms {
    /// The key is expected to be found, otherwise an error is returned
    async fn get_secret(&self, secret: &KeyId, description: &str) -> Result<StoredSecret> {
        self.storage.get(secret).await?.ok_or_else(|| {
            VaultError::EntryNotFound(format!("missing {description} for {secret:?}")).into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::encode;

    #[test]
    fn test_sha256() {
        let digest = VaultKms::sha256(b"a");
        assert_eq!(
            encode(digest),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }
}
