#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_extern_crates,
    unused_parens,
    while_true
)]

//! Implements the Ockam channels interface and provides
//! a C FFI version.
//!
//! Channels are where parties can send messages securely

#![cfg_attr(feature = "nightly", feature(doc_cfg))]

#[macro_use]
extern crate ockam_common;

use ockam_kex::TransportState;
use ockam_vault::Vault;
use std::io::{Read, Write};

/// Represents an Ockam channel for reading and writing payloads
#[derive(Debug)]
pub struct Channel<'a, R: Read, W: Write, V: Vault> {
    transport: TransportState<'a, V>,
    reader: R,
    writer: W,
}

/// Represents the errors that occur within a channel
pub mod error;

