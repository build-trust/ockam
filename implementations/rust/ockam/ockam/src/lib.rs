//! End-to-end encryption and mutual authentication for distributed applications.

#![deny(unsafe_code)]
#![warn(
    missing_docs,
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
pub use ockam_node::{debugger, Context, DelayedEvent, Executor, NodeBuilder, WorkerBuilder};
// ---

mod delay;
mod error;
mod forwarder;
mod metadata;
mod monotonic;
mod system;
mod unique;

pub use error::OckamError;
pub use forwarder::ForwardingService;
pub use metadata::OckamMessage;
pub use system::{SystemBuilder, SystemHandler, WorkerSystem};
pub use unique::unique_with_prefix;

pub mod channel;
pub mod pipe;
pub mod pipe2;
pub mod protocols;
pub mod remote;
pub mod stream;
pub mod workers;

#[cfg(feature="std")]
pub use ockam_abac as abac;
pub use ockam_identity as identity;

pub use ockam_core::{
    allow, deny, errcode, route, Address, Any, AsyncTryClone, Encoded, Error, LocalMessage,
    Mailbox, Mailboxes, Message, Processor, ProtocolId, Result, Route, Routed, TransportMessage,
    Worker,
};

/// Access Control
pub mod access_control {
    pub use ockam_core::access_control::*;
    pub use ockam_identity::access_control::*;
    pub use ockam_node::access_control::*;
}

/// Mark an Ockam Worker implementation.
///
/// This is currently implemented as a re-export of the `async_trait` macro, but
/// may be changed in the future to a [`Worker`](crate::Worker)-specific macro.
pub use ockam_core::worker;

/// Mark an Ockam Processor implementation.
///
/// This is currently implemented as a re-export of the `async_trait` macro, but
/// may be changed in the future to a [`Processor`](crate::Processor)-specific macro.
pub use ockam_core::processor;

// TODO: think about how to handle this more. Probably extract these into an
// `ockam_compat` crate.
pub mod compat {
    //! Compatibility adapter, mostly for `no_std` use.
    //!
    //! Most user code should not use these types.
    pub use ockam_core::compat::*;
    pub use ockam_node::compat::*;
    pub use ockam_node::tokio;
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
    //! Types and traits relating to ockam vaults.
    pub use ockam_core::vault::*;
    pub use ockam_vault::Vault;

    #[cfg(feature = "software_vault_storage")]
    /// Storage
    pub mod storage {
        pub use ockam_vault::storage::*;
    }
}

/// Authenticated Storage
pub mod authenticated_storage {
    pub use ockam_identity::authenticated_storage::mem::*;
    pub use ockam_identity::authenticated_storage::*;
}

#[cfg(feature = "ockam_transport_tcp")]
pub use ockam_transport_tcp::{TcpTransport, TCP};

#[cfg(feature = "ockam_transport_tcp")]
/// Tcp
pub mod tcp {
    pub use ockam_transport_tcp::{InletOptions, OutletOptions};
}
