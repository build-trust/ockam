//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
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

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

// Allow use of logging macros directly.
#[macro_use]
extern crate tracing;

pub use async_trait::async_trait;

#[allow(unused_imports)]
#[macro_use]
/// Re-export of the `async_trait` macro crate.
pub extern crate async_trait;

/// Mark an Ockam Worker implementation.
#[doc(inline)]
pub use async_trait::async_trait as worker;
/// Mark an Ockam Processor implementation.
#[doc(inline)]
pub use async_trait::async_trait as processor;

extern crate ockam_macros;
pub use ockam_macros::{AsyncTryClone, Message};

extern crate futures_util;

/// Access control
pub mod access_control;
pub mod api;
pub mod compat;

/// Debugger
pub mod debugger;
pub mod sessions;
pub mod vault;

/// Encoding
pub mod hex_encoding;

mod cbor_utils;
mod error;
mod key_exchanger;
mod message;
mod processor;
mod routing;
mod type_tag;
mod uint;
mod worker;

pub use access_control::*;
pub use cbor_utils::*;
pub use error::*;
pub use key_exchanger::*;
pub use message::*;
pub use processor::*;
pub use routing::*;
pub use type_tag::*;
pub use uint::*;
pub use worker::*;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
#[doc(hidden)]
pub use compat::println;

#[cfg(feature = "std")]
#[doc(hidden)]
pub use std::println;

use crate::compat::boxed::Box;

/// Clone trait for async structs.
#[async_trait]
pub trait AsyncTryClone: Sized {
    /// Try cloning a object and return an `Err` in case of failure.
    async fn async_try_clone(&self) -> Result<Self>;
}

#[async_trait]
impl<D> AsyncTryClone for D
where
    D: Clone + Sync,
{
    async fn async_try_clone(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

/// Produces Ok(false) to avoid an ambiguous reading from using the unadorned value in auth code.
pub fn deny() -> Result<bool> {
    Ok(false)
}

/// Produces Ok(true) to avoid an ambiguous reading from using the unadorned value in auth code.
pub fn allow() -> Result<bool> {
    Ok(true)
}
