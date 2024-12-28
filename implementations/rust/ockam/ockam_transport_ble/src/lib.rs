//! This crate provides a BLE Transport for Ockam's Routing Protocol.
//! Please read the support [documentation](./documentation.md) for more information.
#![deny(
    //missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(target = "mips", feature(asm))]
#![cfg_attr(target = "mips", feature(asm_experimental_arch))]

#[cfg(feature = "std")]
extern crate core;

#[cfg_attr(feature = "alloc", macro_use)]
extern crate alloc;

#[macro_use]
extern crate tracing;

pub mod driver;
mod error;
mod macros;
mod router;
mod transport;
mod types;
mod workers;

use ockam_core::TransportType;

pub use driver::{BleClient, BleServer};
pub use transport::BleTransport;
pub use types::*;

/// BLE address type constant
pub const BLE: TransportType = TransportType::new(4);
