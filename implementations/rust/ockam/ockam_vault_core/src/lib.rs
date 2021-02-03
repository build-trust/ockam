//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.

// #![no_std] if the std feature is disabled.
#![cfg_attr(not(feature = "std"), no_std)]

mod hash_vault;
pub use hash_vault::*;
mod kid_vault;
pub use kid_vault::*;
pub mod macros;
mod secret;
pub use secret::*;
mod secret_vault;
pub use secret_vault::*;
mod signer_vault;
pub use signer_vault::*;
mod types;
pub use types::*;
mod verifier_vault;
pub use verifier_vault::*;
