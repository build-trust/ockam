use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

use super::super::super::identity::IdentityConstants;
use super::super::super::models::{Identifier, PurposeKeyAttestation};
use super::super::super::storage::{InMemoryStorage, Storage};
use super::super::super::Purpose;
use super::{PurposeKeysReader, PurposeKeysRepository, PurposeKeysWriter};

#[derive(Clone)]
pub struct PurposeKeysStorage {
    storage: Arc<dyn Storage>,
}

#[async_trait]
impl PurposeKeysRepository for PurposeKeysStorage {
    fn as_reader(&self) -> Arc<dyn PurposeKeysReader> {
        Arc::new(self.clone())
    }

    fn as_writer(&self) -> Arc<dyn PurposeKeysWriter> {
        Arc::new(self.clone())
    }
}

impl PurposeKeysStorage {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub fn create() -> Arc<Self> {
        Arc::new(Self::new(InMemoryStorage::create()))
    }

    fn key(purpose: Purpose) -> String {
        let key = match purpose {
            Purpose::SecureChannel => IdentityConstants::SECURE_CHANNEL_PURPOSE_KEY,
            Purpose::Credentials => IdentityConstants::CREDENTIALS_PURPOSE_KEY,
        };

        key.to_string()
    }
}

#[async_trait]
impl PurposeKeysWriter for PurposeKeysStorage {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()> {
        let key = Self::key(purpose);
        self.storage
            .set(
                &subject.to_string(),
                key.to_string(),
                minicbor::to_vec(purpose_key_attestation)?,
            )
            .await
    }

    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()> {
        let key = Self::key(purpose);
        self.storage
            .del(&subject.to_string(), &key.to_string())
            .await
    }
}

#[async_trait]
impl PurposeKeysReader for PurposeKeysStorage {
    async fn retrieve_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>> {
        let key = Self::key(purpose);
        if let Some(data) = self.storage.get(&identifier.to_string(), &key).await? {
            Ok(Some(minicbor::decode(&data)?))
        } else {
            Ok(None)
        }
    }
}
