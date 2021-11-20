//! This crate implements the Pointcheval Saunders signature
//! as described in <https://eprint.iacr.org/2015/525.pdf>
//! and <https://eprint.iacr.org/2017/1197.pdf>
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![allow(clippy::question_mark)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate signature_core;

mod blind_signature;
mod blind_signature_context;
mod issuer;
mod message_generator;
mod pok_signature;
mod pok_signature_proof;
mod prover;
mod public_key;
mod secret_key;
mod signature;
mod verifier;

/// A PS blind signature
pub use blind_signature::*;
/// The blind signature context
pub use blind_signature_context::*;
/// The issuer methods
pub use issuer::*;
/// The generators used for blind signature computation
pub use message_generator::*;
/// The Proof of knowledge of signature proof initial phase
pub use pok_signature::*;
/// The Proof of knowledge of signature proof
pub use pok_signature_proof::*;
/// The proving methods
pub use prover::*;
/// The Pointcheval Saunders public key
pub use public_key::*;
/// The Pointcheval Saunders secret key
pub use secret_key::*;
/// The Pointcheval Saunders signature
pub use signature::*;
/// The verifier methods for validating proofs
pub use verifier::*;

#[cfg(test)]
pub struct MockRng(rand_xorshift::XorShiftRng);

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
