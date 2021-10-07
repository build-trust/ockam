#![cfg_attr(not(feature = "std"), no_std)]

pub use error::TransportError;

mod error;
#[cfg(test)]
mod error_test;
