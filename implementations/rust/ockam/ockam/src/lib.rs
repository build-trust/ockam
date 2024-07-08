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

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
#[macro_use]
extern crate tracing;

pub use error::OckamError;
pub use node::*;
#[cfg(feature = "std")]
pub use ockam_abac as abac;
/// Mark an Ockam Processor implementation.
///
/// This is currently implemented as a re-export of the `async_trait` macro, but
/// may be changed in the future to a [`Processor`](crate::Processor)-specific macro.
pub use ockam_core::processor;
/// Mark an Ockam Worker implementation.
///
/// This is currently implemented as a re-export of the `async_trait` macro, but
/// may be changed in the future to a [`Worker`](crate::Worker)-specific macro.
pub use ockam_core::worker;
pub use ockam_core::{
    allow, deny, errcode, route, Address, Any, AsyncTryClone, Encoded, Error, LocalMessage,
    Mailbox, Mailboxes, Message, Processor, ProtocolId, Result, Route, Routed, TransportMessage,
    Worker,
};
pub use ockam_identity as identity;
// ---
// Export the ockam macros that aren't coming from ockam_core.
pub use ockam_macros::{node, test};
// Export node implementation
#[cfg(feature = "std")]
pub use ockam_node::database::*;
pub use ockam_node::{
    debugger, Context, DelayedEvent, Executor, MessageReceiveOptions, MessageSendReceiveOptions,
    NodeBuilder, WorkerBuilder,
};
#[cfg(feature = "ockam_transport_tcp")]
/// TCP transport
pub mod tcp {
    pub use ockam_transport_tcp::{
        TcpConnection, TcpConnectionMode, TcpConnectionOptions, TcpInletOptions, TcpListener,
        TcpListenerInfo, TcpListenerOptions, TcpOutletOptions, TcpSenderInfo, TcpTransport,
        TcpTransportExtension, MAX_MESSAGE_SIZE, TCP,
    };
}
#[cfg(feature = "ockam_transport_udp")]
/// UDP transport
pub mod udp {
    pub use ockam_transport_udp::{
        UdpBindArguments, UdpBindOptions, UdpPunctureNegotiation, UdpPunctureNegotiationListener,
        UdpPunctureNegotiationListenerOptions, UdpTransport, UdpTransportExtension,
        MAX_MESSAGE_SIZE, UDP,
    };
}
pub use relay_service::{RelayService, RelayServiceOptions};

/// Transport
pub mod transport {
    #[cfg(feature = "std")]
    pub use ockam_transport_core::resolve_peer;

    pub use ockam_transport_core::{
        parse_socket_addr, HostnamePort, StaticHostnamePort, Transport,
    };
}

// ---

// ---

mod error;
mod relay_service;

pub mod remote;

/// Access Control
pub mod access_control {
    pub use ockam_core::access_control::*;
    pub use ockam_identity::secure_channel::access_control::*;
}

/// Flow Controls
pub mod flow_control {
    pub use ockam_core::flow_control::*;
}

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

/// Helper workers
pub mod workers {
    pub use ockam_node::workers::*;
}

#[cfg(feature = "ockam_vault")]
pub mod vault {
    //! Types and traits relating to ockam vaults.
    pub use ockam_vault::*;

    #[cfg(feature = "storage")]
    /// Storage
    pub mod storage {
        pub use ockam_vault::storage::*;
    }
}

/// List of all top-level services
pub mod node;
