//! Software implementation of ockam_vault_core traits.
//!
//! This crate contains one of the possible implementation of the vault traits
//! which you can use with Ockam library.
#![deny(unsafe_code)]
#![warn(
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

pub extern crate ockam_vault_core;

mod asymmetric_impl;
mod error;
mod hasher_impl;
mod key_id_impl;
mod secret_impl;
mod signer_impl;
mod software_vault;
mod symmetric_impl;
mod verifier_impl;
mod xeddsa;

// Re-export types commonly used by higher level APIs
pub use ockam_vault_core::{
    Hasher, KeyIdVault, PublicKey, Secret, SecretAttributes, SecretVault, Signer, Verifier,
};

pub use asymmetric_impl::*;
pub use error::*;
pub use hasher_impl::*;
pub use key_id_impl::*;
pub use secret_impl::*;
pub use signer_impl::*;
pub use software_vault::*;
pub use symmetric_impl::*;
pub use verifier_impl::*;

#[cfg(test)]
#[test]
fn verify_impls() {
    use ockam_vault_core::*;
    fn check_asymmetric_vault<T: AsymmetricVault>() {}
    fn check_hasher<T: Hasher>() {}
    fn check_key_id_vault<T: KeyIdVault>() {}
    fn check_secret_vault<T: SecretVault>() {}
    fn check_signer<T: Signer>() {}
    fn check_symmetric_vault<T: SymmetricVault>() {}
    fn check_verifier<T: Verifier>() {}
    fn check_send<T: Send>() {}
    fn check_sync<T: Sync>() {}
    fn check_static<T: 'static>() {}

    check_asymmetric_vault::<SoftwareVault>();
    check_hasher::<SoftwareVault>();
    check_key_id_vault::<SoftwareVault>();
    check_secret_vault::<SoftwareVault>();
    check_signer::<SoftwareVault>();
    check_symmetric_vault::<SoftwareVault>();
    check_verifier::<SoftwareVault>();
    check_send::<SoftwareVault>();
    check_sync::<SoftwareVault>();
    check_static::<SoftwareVault>();
}
