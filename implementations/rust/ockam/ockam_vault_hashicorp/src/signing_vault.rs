use crate::error::Error;
use base64::Engine;
use ockam_core::compat::rand::random_string;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, Result};
use ockam_vault::{
    EdDSACurve25519PublicKey, EdDSACurve25519Signature, HandleToSecret, Signature, SigningKeyType,
    SigningSecretKeyHandle, VaultError, VaultForSigning, VerifyingPublicKey,
};
use vaultrs::api::transit::requests::CreateKeyRequest;
use vaultrs::api::transit::KeyType;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit::{data, key};

struct HashicorpKeyPair {
    key: SigningSecretKeyHandle,
    public_key: VerifyingPublicKey,
}

/// Security module implementation using an AWS KMS
pub struct HashicorpSigningVault {
    client: VaultClient,

    keys: Arc<RwLock<Vec<HashicorpKeyPair>>>,
}

impl HashicorpSigningVault {
    /// Create a default AWS security module
    pub async fn create() -> Result<Self> {
        // Create a client
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address("http://127.0.0.1:8200")
                .token("dev-only-token")
                .build()
                .unwrap(),
        )
        .unwrap();

        Ok(Self {
            client,
            keys: Default::default(),
        })
    }

    fn key_name_from_handle(handle: &SigningSecretKeyHandle) -> Result<String> {
        Ok(String::from_utf8(handle.handle().value().clone()).unwrap())
    }

    fn handle_from_key_name(key_name: String) -> SigningSecretKeyHandle {
        SigningSecretKeyHandle::EdDSACurve25519(HandleToSecret::new(key_name.into_bytes()))
    }
}

#[async_trait]
impl VaultForSigning for HashicorpSigningVault {
    async fn sign(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
        data: &[u8],
    ) -> Result<Signature> {
        let key_name = Self::key_name_from_handle(signing_secret_key_handle)?;
        let data_base64 = base64::engine::general_purpose::STANDARD.encode(data);
        let signature = data::sign(&self.client, "transit", &key_name, &data_base64, None)
            .await
            .unwrap();
        // TODO: Proper parsing
        let signature = base64::engine::general_purpose::STANDARD
            .decode(&signature.signature[9..])
            .unwrap();

        Ok(Signature::EdDSACurve25519(EdDSACurve25519Signature(
            signature.try_into().unwrap(),
        )))
    }

    async fn generate_signing_secret_key(
        &self,
        signing_key_type: SigningKeyType,
    ) -> Result<SigningSecretKeyHandle> {
        if signing_key_type != SigningKeyType::EdDSACurve25519 {
            return Err(VaultError::InvalidKeyType.into());
        }

        let key_name = random_string();

        key::create(
            &self.client,
            "transit",
            &key_name,
            Some(CreateKeyRequest::builder().key_type(KeyType::Ed25519)),
        )
        .await
        .unwrap();

        let key_handle = Self::handle_from_key_name(key_name.clone());

        let key_info = key::read(&self.client, "transit", &key_name).await.unwrap();

        assert!(key_info.keys.len() == 1);
        let key_info = key_info.keys.into_values().collect::<Vec<_>>();
        let key_info = key_info.first().unwrap();

        let public_key = base64::engine::general_purpose::STANDARD
            .decode(&key_info.public_key)
            .unwrap();

        let public_key = VerifyingPublicKey::EdDSACurve25519(EdDSACurve25519PublicKey(
            public_key.try_into().unwrap(),
        ));

        self.keys.write().unwrap().push(HashicorpKeyPair {
            key: key_handle.clone(),
            public_key,
        });

        Ok(key_handle)
    }

    async fn get_verifying_public_key(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<VerifyingPublicKey> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.key == signing_secret_key_handle {
                    Some(x.public_key.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn get_secret_key_handle(
        &self,
        verifying_public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle> {
        self.keys
            .read()
            .unwrap()
            .iter()
            .find_map(|x| {
                if &x.public_key == verifying_public_key {
                    Some(x.key.clone())
                } else {
                    None
                }
            })
            .ok_or(Error::KeyNotFound.into())
    }

    async fn delete_signing_secret_key(
        &self,
        signing_secret_key_handle: SigningSecretKeyHandle,
    ) -> Result<bool> {
        // TODO: Deleting a key is not allowed by default and requires extra work
        // let key_name = Self::key_name_from_handle(&signing_secret_key_handle)?;
        // key::delete(&self.client, "transit", &key_name)
        //     .await
        //     .unwrap();
        //
        let found = true; // FIXME
        if found {
            self.keys
                .write()
                .unwrap()
                .retain(|x| x.key != signing_secret_key_handle);

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
