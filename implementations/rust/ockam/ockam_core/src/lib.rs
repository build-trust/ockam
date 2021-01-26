// ---
// #![no_std] if the standard library is not present.

#![cfg_attr(not(feature = "std"), no_std)]

mod error;
pub use error::*;
