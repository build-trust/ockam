//!
#![deny(unsafe_code)]
#![warn(
    // TODO restore missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate signature_core;

/// The maximum number of messages that can be signed by this crate
pub const MAX_MSGS: usize = 128;

#[macro_use]
mod util;
mod blind_signature;
mod blind_signature_context;
mod issuer;
mod message_generator;
mod pok_signature;
mod pok_signature_proof;
mod prover;
mod signature;
mod verifier;

pub use blind_signature::*;
pub use blind_signature_context::*;
pub use issuer::*;
pub use message_generator::*;
pub use pok_signature::*;
pub use pok_signature_proof::*;
pub use prover::*;
pub use signature::*;
pub use signature_bls::{ProofOfPossession, PublicKey, SecretKey, Signature as BlsSignature};
#[cfg(test)]
pub use util::MockRng;
pub use verifier::*;
