//! Core types and traits of the Ockam vault.
//!
//! This module contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.

mod asymmetric_vault;
mod hasher;
mod key_id_vault;
mod secret;
mod secret_vault;
mod signer;
mod symmetric_vault;
mod types;
mod verifier;

pub use asymmetric_vault::*;
pub use hasher::*;
pub use key_id_vault::*;
pub use secret::*;
pub use secret_vault::*;
pub use signer::*;
pub use symmetric_vault::*;
pub use types::*;
pub use verifier::*;
