//! This crate implements BLS signatures according to the IETF draft v4
//!
//! for the Proof of Possession Cipher Suite
//!
//! Since BLS signatures can use either G1 or G2 fields, there are two types of
//! public keys and signatures. Normal and Variant (suffix'd with Vt).
//!
//! Normal puts signatures in G1 and pubic keys in G2.
//! Variant is the reverse.
//!
//! This crate has been designed to be compliant with no-std by avoiding allocations
//!
//! but provides some optimizations when an allocator exists for verifying
//! aggregated signatures.

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

mod aggregate_signature;
mod aggregate_signature_vt;
mod multi_public_key;
mod multi_public_key_vt;
mod multi_signature;
mod multi_signature_vt;
mod proof_of_possession;
mod proof_of_possession_vt;
mod public_key;
mod public_key_vt;
mod secret_key;
mod signature;
mod signature_vt;

pub use aggregate_signature::*;
pub use aggregate_signature_vt::*;
pub use multi_public_key::*;
pub use multi_public_key_vt::*;
pub use multi_signature::*;
pub use multi_signature_vt::*;
pub use proof_of_possession::*;
pub use proof_of_possession_vt::*;
pub use public_key::*;
pub use public_key_vt::*;
pub use secret_key::*;
pub use signature::*;
pub use signature_vt::*;

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
