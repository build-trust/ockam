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

mod asymmetric_vault;
mod hasher;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod types;
mod verifier;

/// Storage
pub mod storage;

pub mod test_support;

pub use asymmetric_vault::*;
pub use hasher::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use types::*;
pub use verifier::*;
