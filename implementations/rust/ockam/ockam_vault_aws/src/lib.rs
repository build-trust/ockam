//! AWS implementation of the ockam_vault::Kms trait
//!
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

mod aws_kms_client;
mod aws_signing_vault;
mod error;

pub use aws_kms_client::*;
pub use aws_signing_vault::*;
pub use error::*;
