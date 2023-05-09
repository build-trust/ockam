//! In order to support a variety of cryptographically capable hardware we maintain loose coupling between
//! our protocols and how a specific building block is invoked in a specific hardware.
//! This is achieved using an abstract Vault trait.
//!
//! A concrete implementation of the Vault trait is called an Ockam Vault.
//! Over time, and with help from the Ockam open source community, we plan to add vaults for
//! several TEEs, TPMs, HSMs, and Secure Enclaves.
//!
//! This crate provides a software-only Vault implementation that can be used when no cryptographic
//! hardware is available. The primary Ockam crate uses this as the default Vault implementation.
//!
//! The main [Ockam][main-ockam-crate-link] has optional dependency on this crate.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

/// Storage
#[cfg(feature = "storage")]
pub mod storage;

/// Traits and types defining the behaviour of a Vault
pub mod traits;
/// Default Vault implementation
pub mod vault;

/// Main vault types: PublicKey, Secret, SecretAttributes etc...
mod types;

pub use constants;
pub use traits::*;
pub use types::*;
pub use vault::*;
