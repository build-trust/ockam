use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Error};

use super::super::super::models::{Identifier, PurposeKeyAttestation};
use super::super::super::Purpose;

// TODO: Only one PurposeKey per Purpose per Identity is supported for now

#[async_trait]
pub trait PurposeKeysRepository: PurposeKeysReader + PurposeKeysWriter {
    fn as_reader(&self) -> Arc<dyn PurposeKeysReader>;

    fn as_writer(&self) -> Arc<dyn PurposeKeysWriter>;
}

#[async_trait]
pub trait PurposeKeysWriter: Send + Sync + 'static {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()>;

    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()>;
}

#[async_trait]
pub trait PurposeKeysReader: Send + Sync + 'static {
    async fn retrieve_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>>;

    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<PurposeKeyAttestation> {
        match self.retrieve_purpose_key(identifier, purpose).await? {
            Some(purpose_key) => Ok(purpose_key),
            None => Err(Error::new(
                Origin::Core,
                Kind::NotFound,
                format!("purpose_key not found for identifier {}", identifier),
            )),
        }
    }
}
