//! Core types and traits of the Ockam vault.
//!
//! This crate contains the core types and traits of the Ockam vault and is intended
//! for use by other crates that either provide implementations for those traits,
//! or use traits and types as an abstract dependency.

// #![no_std] if the std feature is disabled.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod hash_vault;
pub mod kid_vault;
pub mod macros;
pub mod secret;
pub mod secret_vault;
pub mod signer_vault;
pub mod types;
pub mod verifier_vault;
