//! Ockam Vault Foreign Function Interface (FFI) for library integration.
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
