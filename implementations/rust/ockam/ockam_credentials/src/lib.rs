//! OCKAM-CREDENTIALS: Implements the structures, traits, and protocols
//! for creating, issuing, and verifying ockam credentials.
//!
//! Ockam credentials are used for authentication and authorization among
//! Ockam compatible connections
#![no_std]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "heapless")]
mod structs {
    pub use core::fmt::{self, Debug, Display};
    use heapless::{consts::*, String, Vec};
    pub type Buffer<T> = Vec<T, U32>;
    pub type ByteString = String<U32>;
}

#[cfg(not(feature = "heapless"))]
mod structs {
    pub use alloc::fmt::{self, Debug, Display};
    use alloc::{string::String, vec::Vec};
    pub type Buffer<T> = Vec<T>;
    pub type ByteString = String;
}

/// The error module
mod error;
/// Helper methods for serializing and deserializing
mod serdes;
pub use error::CredentialError;
/// The attribute types
mod attribute_type;
pub use attribute_type::AttributeType;
/// The attribute struct used in schemas
mod attribute;
pub use attribute::Attribute;
/// Schema used by credentials
mod schema;
pub use schema::Schema;
