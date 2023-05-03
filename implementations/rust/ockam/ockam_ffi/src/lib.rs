//! In order to support a variety of cryptographically capable hardware we maintain loose coupling between our protocols and how a specific building block is invoked in a specific hardware. This is achieved using an abstract Vault trait.
//!
//! A concrete implementation of the Vault trait is called an Ockam Vault. Over time, and with help from the Ockam open source community, we plan to add vaults for several TEEs, TPMs, HSMs, and Secure Enclaves.
//!
//! This crate provides the Vault FFI bindings following the  "C" calling convention, and generates static and dynamic C linkable libraries.
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod error;
mod macros;
mod vault;
mod vault_types;

pub use error::*;
pub use vault::*;
use vault_types::*;
