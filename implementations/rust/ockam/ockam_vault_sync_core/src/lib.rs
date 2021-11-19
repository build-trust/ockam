//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

mod error;
mod vault;
#[cfg(feature = "std")]
mod vault_mutex;
mod vault_sync;
mod vault_worker;

pub use error::*;
pub use vault::*;
#[cfg(feature = "std")]
pub use vault_mutex::*;
pub use vault_sync::*;
pub use vault_worker::*;
