#![cfg_attr(not(feature = "std"), no_std)]

pub use error::TransportError;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

mod error;
#[cfg(test)]
mod error_test;
pub mod tcp;

/// TCP address type constant
pub const TCP: u8 = 1;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.tcp";
