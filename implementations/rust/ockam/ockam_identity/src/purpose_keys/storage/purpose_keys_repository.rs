use crate::models::{Identifier, PurposeKeyAttestation};
use crate::Purpose;
use async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::Result;
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;

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

#[cfg(feature = "std")]
#[async_trait]
impl<T: PurposeKeysRepository> PurposeKeysRepository for AutoRetry<T> {
    async fn set_purpose_key(
        &self,
        subject: &Identifier,
        purpose: Purpose,
        purpose_key_attestation: &PurposeKeyAttestation,
    ) -> Result<()> {
        retry!(self
            .wrapped
            .set_purpose_key(subject, purpose, purpose_key_attestation))
    }

    async fn delete_purpose_key(&self, subject: &Identifier, purpose: Purpose) -> Result<()> {
        retry!(self.wrapped.delete_purpose_key(subject, purpose))
    }

    async fn get_purpose_key(
        &self,
        identifier: &Identifier,
        purpose: Purpose,
    ) -> Result<Option<PurposeKeyAttestation>> {
        retry!(self.wrapped.get_purpose_key(identifier, purpose))
    }

    async fn delete_all(&self) -> Result<()> {
        retry!(self.wrapped.delete_all())
    }
}
