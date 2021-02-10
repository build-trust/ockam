//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.

#![no_std]
#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

mod asymmetric_vault;
mod hash_vault;
mod key_id_vault;
mod macros;
mod secret;
mod secret_vault;
mod signer_vault;
mod symmetric_vault;
mod types;
mod verifier_vault;

pub use asymmetric_vault::*;
pub use hash_vault::*;
pub use key_id_vault::*;
pub use macros::*;
pub use secret::*;
pub use secret_vault::*;
pub use signer_vault::*;
pub use symmetric_vault::*;
pub use types::*;
pub use verifier_vault::*;
