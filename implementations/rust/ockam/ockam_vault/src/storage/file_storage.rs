use ockam_core::compat::boxed::Box;
use ockam_core::vault::storage::Storage;
use ockam_core::vault::{KeyId, SecretKey};
use ockam_core::{async_trait, Result};

pub struct FileStorage {}

#[async_trait]
impl Storage for FileStorage {
    async fn store(&self, _secret: &KeyId, _key: &SecretKey) -> Result<()> {
        todo!()
    }

    async fn load(&self, _secret: &KeyId) -> Result<SecretKey> {
        todo!()
    }

    async fn delete(&self, _secret: &KeyId) -> Result<SecretKey> {
        todo!()
    }
}
