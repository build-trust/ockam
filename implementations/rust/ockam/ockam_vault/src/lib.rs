//! Software implementation of ockam_core::vault traits.
//!
//! This crate contains one of the possible implementation of the vault traits
//! which you can use with Ockam library.
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

/// Storage
#[cfg(feature = "storage")]
pub mod storage;

/// Service
#[cfg(feature = "service")]
pub mod service;

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
