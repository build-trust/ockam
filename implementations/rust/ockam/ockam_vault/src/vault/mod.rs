//! Core types and traits of the Ockam vault.
//!
//! This module contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.
//!
//! # Examples
//!
//! See the [`ockam_vault`] crate for usage examples.
//!
//! [`ockam_vault`]: https://docs.rs/ockam_vault/latest

mod asymmetric_impl;
mod secrets_store_impl;
mod signer_impl;
mod symmetric_impl;
#[allow(clippy::module_inception)]
mod vault;
mod vault_builder;
mod vault_error;
mod vault_kms;

pub use vault::*;
pub use vault_builder::*;
pub use vault_error::*;
pub use vault_kms::*;
