//! This crate supports the domain of "identities", which is required to create secure channels:
//!
//!  - the `identity` module describes an entity as a set of verified key changes and an identifier
//!    uniquely representing those changes
//!
//!  - the `identities` module provides services to create, update, and import identities
//!
//!  - the `credential` module describes sets of attributes describing a given identity and signed by
//!    another identity
//!
//!  - the `credentials` module provides services to create, import and verify credentials
//!
//!  - the `secure_channel` module describes the steps required to establish a secure channel
//!    between 2 identities
//!
//!  - the `secure_channels` module provides services to create a secure channel between 2 identities

#![deny(unsafe_code)]
#![warn(
// prevented by big_array
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_import_braces,
unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

/// Data types supporting the creation of a credential
pub mod credential;

/// Services for creating and validating credentials
pub mod credentials;

/// Service for the management of identities
pub mod identities;

/// Data types representing an identity
pub mod identity;

/// Data types supporting the creation of a secure channels
pub mod secure_channel;

/// Service supporting the creation of secure channel listener and connection to a listener
pub mod secure_channels;

///
/// Exports
///
pub use credential::*;
pub use credentials::*;
pub use identities::*;
pub use identity::*;
pub use secure_channel::*;
pub use secure_channels::*;
