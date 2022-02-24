//! Ockam is a library for building devices that communicate securely, privately
//! and trustfully with cloud services and other devices.
//!
//! For a comprehensive introduction to Ockam, see <https://www.ockam.io/learn>.
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

// ---
// Export the ockam macros that aren't coming from ockam_core.
pub use ockam_macros::{node, test};
// ---

// Export node implementation
pub use ockam_node::{start_node, Context};
// ---

mod delay;
mod error;
mod forwarder;
mod lease;
mod metadata;
mod monotonic;
mod protocols;
mod system;
mod unique;

pub use delay::DelayedEvent;
pub use error::OckamError;
pub use forwarder::ForwardingService;
pub use lease::Lease;
pub use metadata::OckamMessage;
pub use system::{SystemHandler, WorkerSystem};
pub use unique::unique_with_prefix;

pub mod channel;
pub mod pipe;
pub mod remote_forwarder;
pub mod stream;
pub mod workers;

pub use ockam_identity as identity;

pub use ockam_core::{
    route, worker, Address, Any, AsyncTryClone, Encoded, Error, LocalMessage, Message, ProtocolId,
    Result, Route, Routed, RouterMessage, TransportMessage, Worker,
};

// TODO: think about how to handle this more. Probably extract these into an
// `ockam_compat` crate.
pub mod compat {
    //! Compatibility adapter for
    pub use ockam_core::compat::*;
    pub use ockam_node::compat::*;
}

// TODO: these next few modules should be rethought when we do the updates for
// getting the layer 2 crates to GA, but for now they just move things out of
// the way.

pub mod key_exchange {
    //! Module containing types required for key exchange.
    pub use ockam_key_exchange_core::NewKeyExchanger;
    #[cfg(feature = "noise_xx")]
    pub use ockam_key_exchange_xx::XXNewKeyExchanger;
}

#[cfg(feature = "ockam_vault")]
pub mod vault {
    //! Types relating to ockam vaults.
    // TODO: define what an "ockam vault" is in this documentation.
    pub use ockam_vault::SoftwareVault;
    pub use ockam_vault_sync_core::Vault;
}

#[cfg(feature = "ockam_transport_tcp")]
pub use ockam_transport_tcp::{TcpTransport, TCP};
