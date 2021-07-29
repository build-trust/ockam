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
mod hasher_impl;
mod key_id_impl;
mod secret_impl;
mod signer_impl;
mod software_vault;
mod symmetric_impl;
mod verifier_impl;
mod xeddsa;
mod secret_key_share_impl;

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
pub use secret_key_share_impl::*;

#[cfg(test)]
struct MockRng(rand_xorshift::XorShiftRng);

#[cfg(test)]
impl rand_core::SeedableRng for MockRng {
    type Seed = [u8; 16];

    fn from_seed(seed: Self::Seed) -> Self {
        Self(rand_xorshift::XorShiftRng::from_seed(seed))
    }
}

#[cfg(test)]
impl rand_core::CryptoRng for MockRng {}

#[cfg(test)]
impl rand_core::RngCore for MockRng {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.0.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.0.try_fill_bytes(dest)
    }
}