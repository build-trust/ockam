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
mod aws;
mod error;
mod hasher_impl;
mod secret_impl;
mod signer_impl;
mod symmetric_impl;
mod vault;
mod verifier_impl;
mod xeddsa;

pub use aws::*;
pub use error::*;
pub use vault::*;
