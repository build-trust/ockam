//! Identity is an abstraction over Identitys and Vaults, easing the use of these primitives in
//! authentication and authorization APIs.
#![deny(unsafe_code)]
#![warn(
    // prevented by big_array
    //  missing_docs,
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

use ockam_core::AsyncTryClone;
use ockam_vault::{Hasher, SecretVault, Signer, Verifier};

use crate::IdentityError;

pub mod authenticated_storage;
pub mod change;
pub mod change_history;
pub mod credential;

pub mod error;

pub use error::*;

mod channel;
mod identifiers;
mod identity;
mod identity_builder;
mod key_attributes;
mod public_identity;

pub use channel::*;
pub use identifiers::*;
pub use identity::*;
pub use identity_builder::*;
pub use key_attributes::*;
pub use public_identity::*;

mod signature;

#[cfg(test)]
mod invalid_signatures_tests;

/// Traits required for a Vault implementation suitable for use in an Identity
pub trait IdentityVault:
    SecretVault + SecureChannelVault + Hasher + Signer + Verifier + AsyncTryClone + Send + 'static
{
}

impl<D> IdentityVault for D where
    D: SecretVault
        + SecureChannelVault
        + Hasher
        + Signer
        + Verifier
        + AsyncTryClone
        + Send
        + 'static
{
}
