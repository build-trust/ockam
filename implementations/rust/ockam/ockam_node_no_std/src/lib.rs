//! no_std implementation for ockam_node
//!
//! This crate contains a first draft of the missing functionality
//! from std which is required by ockam_node on no_std targets.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

/// no_std implementation of tokio functionality required by ockam_node
pub mod tokio;

/// Placeholder Mutex implementation for use on cortex_m platforms
///
/// TODO Provide a std::sync::Mutex compatible mutex interface and
/// abstract the implementations into the target specific crates.
pub mod interrupt {
    #[cfg(feature = "no_std")]
    pub use cortex_m::interrupt::{free, Mutex};
}
