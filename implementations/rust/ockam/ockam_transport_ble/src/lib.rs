//! Bluetooth Low Energy (BLE) Transport for Ockam's routing framework
//!
//! The `ockam_node` (or `ockam_node_no_std`) crate sits at the core
//! of the Ockam routing framework, with transport specific
//! abstraction plugins.  This crate implements a Bluetooth connection
//! plugin for this architecture.
//!
//! You can use Ockam's routing mechanism for cryptographic protocols,
//! key lifecycle, credential exchange, enrollment, etc, without having
//! to worry about the transport specifics.
//!

#![deny(
    //missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
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

pub use driver::{BleClient, BleServer};
pub use transport::BleTransport;
pub use types::*;

/// BLE address type constant
pub const BLE: u8 = 4;

pub(crate) const CLUSTER_NAME: &str = "_internals.transport.ble";
