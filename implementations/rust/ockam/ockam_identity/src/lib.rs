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

pub use channel::*;
pub use error::*;
pub use identifiers::*;
pub use identity::*;
pub use identity::*;
pub use identity_builder::*;
pub use key_attributes::*;
use ockam_channel::SecureChannelVault;
use ockam_core::compat::{collections::HashMap, string::String};
use ockam_core::AsyncTryClone;
use ockam_vault::{Hasher, SecretVault, Signer, Verifier};
pub use worker::*;

use crate::IdentityError;

pub mod authenticated_storage;
pub mod change;
pub mod change_history;
mod channel;
pub mod error;
mod identifiers;
mod identity;
mod identity_builder;
mod key_attributes;
mod signature;
mod worker;

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

/// Identity event attributes
pub type IdentityEventAttributes = HashMap<String, String>;
