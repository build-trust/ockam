use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;

use crate::models::{Identifier, PurposeKeyAttestation};
use crate::Purpose;

// TODO: Only one PurposeKey per Purpose per Identity is supported for now

/// This repository stores [`super::super::super::purpose_key::PurposeKey`]s
#[async_trait]
pub trait PurposeKeysRepository: Send + Sync + 'static {
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

    /// Retrieve the [`super::super::super::purpose_key::PurposeKey`]
    /// for given [`Identifier`] and [`Purpose`]
    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>>;

    /// Delete all keys
    async fn delete_all(&self) -> Result<()>;
}
