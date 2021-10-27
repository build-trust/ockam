//! ockam_node - Ockam Node API
#![deny(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;

#[cfg_attr(feature = "std", macro_use)]
extern crate tracing;

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate ockam_executor;

#[cfg(not(feature = "std"))]
pub use ockam_executor::tokio;

#[cfg(feature = "std")]
pub use tokio;

mod cancel;
mod context;
mod error;
mod executor;
mod handle;
mod messages;
mod node;
mod parser;
mod relay;
mod router;
mod tests;

pub use cancel::*;
pub use context::*;
pub use executor::*;
pub use handle::*;
pub use messages::*;

pub use node::{start_node, NullWorker};

#[cfg(feature = "std")]
use core::future::Future;
#[cfg(feature = "std")]
use tokio::{runtime::Runtime, task};

/// Execute a future without blocking the executor
///
/// This is a wrapper around two simple tokio functions to allow
/// ockam_node to wait for a task to be completed in a non-async
/// environment.
///
/// This function is not meant to be part of the ockam public API, but
/// as an implementation utility for other ockam utilities that use
/// tokio.
#[doc(hidden)]
#[cfg(feature = "std")]
pub fn block_future<F>(rt: &Runtime, f: F) -> <F as Future>::Output
where
    F: Future + Send,
    F::Output: Send,
{
    task::block_in_place(move || {
        let local = task::LocalSet::new();
        local.block_on(rt, f)
    })
}

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
