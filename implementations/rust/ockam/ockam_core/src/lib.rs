//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.

#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]
// #![no_std] if the std feature is disabled.
#![cfg_attr(not(feature = "std"), no_std)]

mod address;
mod error;
mod message;
mod worker;

pub use address::*;
pub use error::*;
pub use message::*;
pub use worker::*;
