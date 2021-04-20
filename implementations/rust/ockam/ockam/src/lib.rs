// ---
// #![no_std] if the standard library is not present.

#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate serde_big_array;

big_array! { BigArray; 96 }

// ---
// Export the #[node] attribute macro.

pub use ockam_node_attribute::*;

// ---
// Export node implementation

#[cfg(all(feature = "std", feature = "ockam_node"))]
pub use ockam_node::*;

#[cfg(all(not(feature = "std"), feature = "ockam_node_no_std"))]
pub use ockam_node_no_std::*;

// ---

mod profile;
pub use profile::*;
mod error;
pub use error::*;
mod credential;
mod lease;

pub use credential::*;
pub use lease::*;

mod remote_forwarder;
pub use remote_forwarder::*;

pub use ockam_core::async_trait::async_trait as async_worker;
pub use ockam_core::{
    Address, Any, Encoded, Error, Message, Result, Route, Routed, RouterMessage, TransportMessage,
    Worker,
};
pub use ockam_vault_sync_core::Vault;

pub use ockam_channel::SecureChannel;

#[cfg(feature = "ockam_vault")]
pub use ockam_vault::SoftwareVault;

#[cfg(feature = "ockam_transport_tcp")]
pub use ockam_transport_tcp::{TcpTransport, TCP};
