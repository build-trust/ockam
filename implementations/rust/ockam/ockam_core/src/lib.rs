//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!(r#"Cannot compile both features "std" and "alloc""#);

#[cfg(all(not(feature = "std"), not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

pub extern crate hashbrown;

#[allow(unused_imports)]
#[macro_use]
pub extern crate hex;

#[allow(unused_imports)]
#[macro_use]
pub extern crate async_trait;
pub use async_trait::async_trait as worker;

pub mod compat;
mod error;
mod message;
mod processor;
mod routing;
mod worker;

pub use error::*;
pub use message::*;
pub use processor::*;
pub use routing::*;
pub use worker::*;

#[cfg(feature = "std")]
pub use std::println;
#[cfg(all(not(feature = "std"), feature = "alloc"))]
/// println macro for no_std
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        // TODO replace with defmt or similiar
        cortex_m_semihosting::hprintln!($($arg)*).unwrap();
    }};
}
