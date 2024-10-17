//! This crate contains the core types of the [Ockam][main-ockam-crate-link]
//! library and is intended for use by crates that provide features and add-ons
//! to the main [Ockam][main-ockam-crate-link] library.
//!
//! The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
//! this crate.
//!
//! ## Crate Features
//!
//! The `ockam_core` crate has a Cargo feature named `"std"` that is enabled by
//! default. In order to use this crate in a `no_std` context this feature can
//! be disabled as follows
//!
//! ```toml
//! [dependencies]
//! ockam_core = { version = "<current version>" , default-features = false }
//! ```
//!
//! Please note that Cargo features are unioned across the entire dependency
//! graph of a project. If any other crate you depend on has not opted out of
//! `ockam_core` default features, Cargo will build `ockam_core` with the std
//! feature enabled whether or not your direct dependency on `ockam_core`
//! has `default-features = false`.
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
pub mod flow_control;

/// Encoding
pub mod hex_encoding;

/// Environmental variables
#[cfg(feature = "std")]
pub mod env;

pub mod bare;
mod cbor;
mod error;
mod identity;
mod message;
mod processor;
mod routing;
mod uint;
mod worker;

pub use access_control::*;
pub use cbor::*;
pub use error::*;
pub use identity::*;
pub use message::*;
pub use processor::*;
pub use routing::*;
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
#[inline]
pub fn deny() -> Result<bool> {
    Ok(false)
}

/// Produces Ok(true) to avoid an ambiguous reading from using the unadorned value in auth code.
#[inline]
pub fn allow() -> Result<bool> {
    Ok(true)
}
