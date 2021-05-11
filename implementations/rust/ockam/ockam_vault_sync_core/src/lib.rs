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

mod error;
mod vault;
mod vault_mutex;
mod vault_sync;
mod vault_worker;

pub use error::*;
pub use vault::*;
pub use vault_mutex::*;
pub use vault_sync::*;
pub use vault_worker::*;
