//! Implements the Ockam vault interface and provides
//! a C FFI version.
//!
//! Vault represents a location where cryptographic keys live such
//! as secure enclaves, TPMs, HSMs, Keyrings, files, memory, etc.

#![deny(
missing_docs,
trivial_casts,
trivial_numeric_casts,
unconditional_recursion,
unused_import_braces,
unused_lifetimes,
unused_qualifications,
unused_extern_crates,
unused_parens,
while_true
)]

/// Represents the errors that occur within a vault
pub mod error;
#[cfg(feature = "ffi")]
/// The ffi functions, structs, and constants
pub mod ffi;
