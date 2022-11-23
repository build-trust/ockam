//! Secure channel types and traits of the Ockam library.
//!
//! This crate contains the secure channel types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
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

mod common;
mod error;
mod local_info;
mod secure_channel;
mod secure_channel_decryptor;
mod secure_channel_encryptor;
mod secure_channel_listener;
mod traits;

pub use common::*;
pub use error::*;
pub use local_info::*;
pub use secure_channel::*;
pub use secure_channel_decryptor::*;
pub(crate) use secure_channel_encryptor::*;
pub use secure_channel_listener::*;
pub use traits::*;
