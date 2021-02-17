//! Lease - metadata for securely managing secrets
//!
//! This crate provides the ability to manage leases
//!
//! A lease is metadata about a secret that indicates a validity period,
//! renewability, and tags. The use case is wrapping a token to an external service
//! which doesn’t support such features. Leases will wrap the token such that the
//! token is guaranteed to be valid while the lease is valid.
//! A lease is revoked at the end of the time duration.
//! Some may support renewals such as extending the validity period.
//! A lease can be revoked at any time by the issuing party.
//!
//! The main Ockam crate re-exports types defined in this crate.
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

mod lease;
mod lease_signature;

pub use lease::*;
pub use lease_signature::*;
