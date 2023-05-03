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

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

pub use ockam_core;

mod asymmetric_impl;
mod error;
mod hasher_impl;
mod secret_impl;
mod signer_impl;

/// AWS KMS
#[cfg(feature = "aws")]
pub mod aws;

/// Storage
#[cfg(feature = "storage")]
pub mod storage;

mod symmetric_impl;
mod vault;
mod verifier_impl;
mod xeddsa;

// Re-export types commonly used by higher level APIs
pub use ockam_core::vault::{
    Hasher, KeyId, PublicKey, SecretAttributes, SecretVault, Signer, Verifier,
};

pub use asymmetric_impl::*;
pub use error::*;
pub use hasher_impl::*;
pub use secret_impl::*;
pub use signer_impl::*;
pub use symmetric_impl::*;
pub use vault::*;
pub use verifier_impl::*;
