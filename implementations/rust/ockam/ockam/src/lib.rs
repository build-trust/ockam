//! Ockam is a library for building devices that communicate securely, privately
//! and trustfully with cloud services and other devices.
#![deny(unsafe_code)]
#![warn(
    // missing_docs // not compatible with big_array
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

#[macro_use]
extern crate tracing;

#[allow(unused_imports)]
#[macro_use]
pub extern crate hex;

// ---
// Export the ockam macros that aren't coming from ockam_core.
pub use ockam_macros::{node, test};
// ---

// Export node implementation
pub use ockam_node::*;
// ---

mod delay;
mod error;
mod forwarder;
mod lease;
mod monotonic;
mod protocols;
mod remote_forwarder;
mod unique;

pub use delay::*;
pub use error::*;
pub use forwarder::*;
pub use lease::*;
pub use ockam_core::compat;
pub use ockam_core::println;
pub use ockam_core::worker;
pub use ockam_core::AsyncTryClone;
pub use ockam_entity::*;
pub use protocols::*;
pub use remote_forwarder::*;
pub use unique::*;

pub mod channel;
pub mod pipe;
pub mod stream;

pub use ockam_core::{
    Address, Any, Encoded, Error, LocalMessage, Message, ProtocolId, Result, Route, Routed,
    RouterMessage, TransportMessage, Worker,
};

pub use ockam_channel::SecureChannel;

pub use ockam_key_exchange_core::NewKeyExchanger;

pub use ockam_core::route;

#[cfg(feature = "noise_xx")]
pub use ockam_key_exchange_xx::XXNewKeyExchanger;

#[cfg(feature = "ockam_vault")]
pub use ockam_vault_sync_core::{Vault, VaultSync};

#[cfg(feature = "ockam_vault")]
pub use ockam_vault::SoftwareVault;

#[cfg(feature = "ockam_transport_tcp")]
pub use ockam_transport_tcp::{TcpTransport, TCP};
