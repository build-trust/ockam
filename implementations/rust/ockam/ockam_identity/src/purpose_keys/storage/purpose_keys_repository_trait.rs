use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_core::{async_trait, Error};

use crate::models::{Identifier, PurposeKeyAttestation};
use crate::Purpose;

// TODO: Only one PurposeKey per Purpose per Identity is supported for now

/// Storage for [`super::super::super::purpose_key::PurposeKey`]s
#[async_trait]
pub trait PurposeKeysRepository: PurposeKeysReader + PurposeKeysWriter {
    /// Return the read access to the Storage
    fn as_reader(&self) -> Arc<dyn PurposeKeysReader>;
    /// Return the write access to the Storage
    fn as_writer(&self) -> Arc<dyn PurposeKeysWriter>;
}

/// Write access to [`super::super::super::purpose_key::PurposeKey`]s' Storage
#[async_trait]
pub trait PurposeKeysWriter: Send + Sync + 'static {
    /// Set the [`super::super::super::purpose_key::PurposeKey`]
    /// for given [`Identifier`] and [`Purpose`] overwriting existing one (if any)
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()>;

    /// Delete the [`super::super::super::purpose_key::PurposeKey`]
    /// for given [`Identifier`] and [`Purpose`]
    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()>;
}

/// Read access to [`super::super::super::purpose_key::PurposeKey`]s' Storage
#[async_trait]
pub trait PurposeKeysReader: Send + Sync + 'static {
    /// Retrieve the [`super::super::super::purpose_key::PurposeKey`]
    /// for given [`Identifier`] and [`Purpose`]
    async fn retrieve_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>>;

    /// Get the [`super::super::super::purpose_key::PurposeKey`]
    /// for given [`Identifier`] and [`Purpose`]
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
