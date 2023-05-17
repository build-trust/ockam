//! This crate provides an implementation of an Ockam [Ockam][main-ockam-crate-link]
//! Node and is intended for use by crates that provide features and add-ons
//! to the main [Ockam][main-ockam-crate-link] library.
//!
//! The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
//! this crate, when the `"std"` feature is enabled.
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    dead_code,
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

#[cfg(not(feature = "std"))]
pub use ockam_executor::tokio;

#[cfg(feature = "std")]
pub use tokio;

/// Async Mutex and RwLock
pub mod compat;

/// MPSC channel type aliases
pub mod channel_types;

#[cfg(feature = "metrics")]
mod metrics;

/// Api helpers
pub mod api;

/// Debugger
pub mod debugger;

/// Callback utility
pub mod callback;

mod async_drop;
mod context;
mod delayed;
mod error;
mod executor;
mod messages;
mod node;
mod parser;
mod processor_builder;
mod relay;
mod router;
mod rpc_client;
mod worker_builder;

pub use context::*;
pub use delayed::*;
pub use error::*;
pub use executor::*;
pub use messages::*;
pub use processor_builder::ProcessorBuilder;
pub use rpc_client::*;
pub use worker_builder::WorkerBuilder;

pub use node::{NodeBuilder, NullWorker};

#[cfg(feature = "std")]
use core::future::Future;
#[cfg(feature = "std")]
use tokio::task;

#[doc(hidden)]
#[cfg(feature = "std")]
pub fn spawn<F: 'static>(f: F)
where
    F: Future + Send,
    F::Output: Send,
{
    task::spawn(f);
}

#[cfg(not(feature = "std"))]
pub use crate::tokio::runtime::{block_future, spawn};

// pub(crate) mod error {
//     //! Move this module to its own file eventually
//     //!
//     //! Utility module to construct various error types

//     use crate::messages::RouterError;
//     use crate::tokio::sync::mpsc::error::SendError;
//     use core::fmt::Debug;
//     #[cfg(feature = "std")]
//     use ockam_core::compat::error::Error as StdError;
//     use ockam_core::{
//         errcode::{Kind, Origin},
//         Error,
//     };

//     impl From<RouterError> for Error {
//         #[track_caller]
//         fn from(e: RouterError) -> Error {
//             Error::new(Origin::Node, Kind::Internal, e)
//         }
//     }

//     #[track_caller]
//     pub fn from_send_err<T: Debug + Send + Sync + 'static>(e: SendError<T>) -> Error {
//         node_internal(e)
//     }

//     #[track_caller]
//     #[cfg(feature = "std")]
//     pub fn from_elapsed(e: tokio::time::error::Elapsed) -> Error {
//         Error::new(Origin::Node, Kind::Timeout, e)
//     }

//     #[track_caller]
//     #[cfg(feature = "std")]
//     pub fn node_internal(e: impl StdError + Send + Sync + 'static) -> Error {
//         Error::new(Origin::Node, Kind::Internal, e)
//     }

//     #[track_caller]
//     pub fn node_without_cause(kind: Kind) -> Error {
//         Error::new_without_cause(Origin::Node, kind)
//     }

//     #[track_caller]
//     pub fn internal_without_cause() -> Error {
//         Error::new_without_cause(Origin::Node, Kind::Internal)
//     }

//     #[cfg(not(feature = "std"))]
//     #[track_caller]
//     pub fn node_internal<E>(_e: E) -> Error {
//         Error::new_without_cause(Origin::Node, Kind::Internal)
//     }

//     #[cfg(not(feature = "std"))]
//     #[track_caller]
//     pub fn from_elapsed<E>(_e: E) -> Error {
//         Error::new_without_cause(Origin::Node, Kind::Timeout)
//     }
// }
