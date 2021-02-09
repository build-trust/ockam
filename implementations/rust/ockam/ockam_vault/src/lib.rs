//! Software implementation of ockam_vault_core traits.
//!
//! This crate contains one of the possible implementation of the vault traits
//! which you can use with Ockam library.

#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

pub extern crate ockam_vault_core;

mod error;
mod hash_impl;
mod key_id_impl;
mod secret_impl;
mod signer_impl;
mod software_vault;
mod verifier_impl;
mod xeddsa;

pub use error::*;
pub use hash_impl::*;
pub use key_id_impl::*;
pub use secret_impl::*;
pub use signer_impl::*;
pub use software_vault::*;
pub use verifier_impl::*;
