#![allow(missing_docs, dead_code)]

/// Utilities
mod utils;

/// Errors
mod error;

/// On-the-wire data types
pub mod models;

/// Service for the management of identities
pub mod identities;

/// Data types representing an identity
pub mod identity;

/// Purpose keys
pub mod purpose_keys;

/// Purpose key
pub mod purpose_key;

/// Services for creating and validating credentials
pub mod credentials;

/// Data types supporting the creation of a secure channels
pub mod secure_channel;

/// Service supporting the creation of secure channel listener and connection to a listener
pub mod secure_channels;

///
/// Exports
///
pub use credentials::*;
pub use error::*;
pub use identities::*;
pub use identity::*;
pub use purpose_key::*;
pub use purpose_keys::*;
pub use secure_channel::*;
pub use secure_channels::*;
