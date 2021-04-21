//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.

#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

mod asymmetric_vault;
mod error;
mod hasher;
mod key_id_vault;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod vault;
mod vault_sync;
mod verifier;

pub use asymmetric_vault::*;
pub use error::*;
pub use hasher::*;
pub use key_id_vault::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use vault::*;
pub use vault_sync::*;
pub use verifier::*;

#[cfg(feature = "software_vault")]
impl Vault {
    /// Start a Vault with SoftwareVault implementation.
    pub async fn create(ctx: &Context) -> Result<Address> {
        Self::create_with_inner(ctx, SoftwareVault::default()).await
    }
}
