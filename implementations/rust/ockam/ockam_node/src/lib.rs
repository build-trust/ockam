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

/// Support for storing persistent values
pub mod storage;
mod worker_builder;

pub use context::*;
pub use delayed::*;
pub use error::*;
pub use executor::*;
pub use messages::*;
pub use processor_builder::ProcessorBuilder;
pub use storage::*;
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
