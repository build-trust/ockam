//! Software implementation of ockam_vault_core traits.
//!
//! This crate contains one of the possible implementation of the vault traits
//! which you can use with Ockam library.

#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

pub extern crate ockam_vault_core;

mod asymmetric_impl;
mod error;
mod error_impl;
mod hasher_impl;
mod key_id_impl;
mod secret_impl;
mod signer_impl;
mod software_vault;
mod symmetric_impl;
mod verifier_impl;
mod xeddsa;

pub use asymmetric_impl::*;
pub use error::*;
pub use error_impl::*;
pub use hasher_impl::*;
pub use key_id_impl::*;
pub use secret_impl::*;
pub use signer_impl::*;
pub use software_vault::*;
pub use symmetric_impl::*;
pub use verifier_impl::*;
