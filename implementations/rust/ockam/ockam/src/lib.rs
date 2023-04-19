//! End-to-end encrypted, mutually authenticated, secure communication.
//!
//! _[A hands-on guide 👉][e2ee-rust-guide]_.
//!
//! Data, within modern distributed applications, are rarely exchanged over a single point-to-point
//! transport connection. Application messages routinely flow over complex, multi-hop, multi-protocol
//! routes — _across data centers, through queues and caches, via gateways and brokers_ — before reaching
//! their end destination.
//!
//! Transport layer security protocols are unable to protect application messages because their protection
//! is constrained by the length and duration of the underlying transport connection.
//!
//! Ockam makes it simple for our applications to guarantee end-to-end integrity, authenticity,
//! and confidentiality of data. We no longer have to implicitly depend on the defenses of every machine
//! or application within the same, usually porous, network boundary. Our application's messages don't have
//! to be vulnerable at every point, along their journey, where a transport connection terminates.
//!
//! Instead, our application can have a strikingly smaller vulnerability surface and easily make
//! _granular authorization decisions about all incoming information and commands._
//!
//! ## Features
//!
//! * End-to-end encrypted, mutually authenticated _secure channels_.
//! * Multi-hop, multi-transport, application layer routing.
//! * Key establishment, rotation, and revocation - _for fleets, at scale_.
//! * Lightweight, Concurrent, Stateful Workers that enable _simple APIs_.
//! * Attribute-based Access Control - credentials with _selective disclosure_.
//! * Add-ons for a variety of operating environments, transport protocols, and _cryptographic hardware_.
//!
//! ## Documentation
//!
//! Tutorials, examples and reference guides are available at [docs.ockam.io](https://docs.ockam.io).
//!
//! [e2ee-rust-guide]: https://docs.ockam.io/reference/libraries/rust

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
pub use ockam_node::{
    debugger, Context, DelayedEvent, Executor, MessageReceiveOptions, MessageSendReceiveOptions,
    NodeBuilder, WorkerBuilder,
};
// ---

mod delay;
mod error;
mod forwarding_service;
mod metadata;
mod monotonic;
mod system;
mod unique;

pub use error::OckamError;
pub use forwarding_service::{ForwardingService, ForwardingServiceOptions};
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

#[cfg(feature = "std")]
pub use ockam_abac as abac;
pub use ockam_identity as identity;
#[cfg(feature = "std")]
pub use ockam_identity::storage::lmdb_storage::*;

pub use ockam_core::{
    allow, deny, errcode, route, Address, Any, AsyncTryClone, Encoded, Error, LocalMessage,
    Mailbox, Mailboxes, Message, Processor, ProtocolId, Result, Route, Routed, TransportMessage,
    Worker,
};

/// Access Control
pub mod access_control {
    pub use ockam_core::access_control::*;
    pub use ockam_identity::secure_channel::access_control::*;
}

/// Flow Controls
pub mod flow_control {
    pub use ockam_core::flow_control::*;
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
    pub use ockam_core::NewKeyExchanger;
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

#[cfg(feature = "ockam_transport_tcp")]
pub use ockam_transport_tcp::{
    TcpConnectionOptions, TcpInletOptions, TcpListenerOptions, TcpOutletOptions, TcpTransport,
};

/// List of all top-level services
pub mod node;

pub use node::*;
