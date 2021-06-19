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

#[macro_use]
extern crate tracing;

mod context;
mod error;
mod executor;
mod mailbox;
mod messages;
mod node;
mod parser;
mod relay;
mod router;

pub use context::*;
pub use executor::*;
pub use mailbox::*;
pub use messages::*;

pub use node::start_node;

use std::future::Future;
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
pub fn block_future<'r, F>(rt: &'r Runtime, f: F) -> <F as Future>::Output
where
    F: Future + Send,
    F::Output: Send,
{
    task::block_in_place(move || {
        let local = task::LocalSet::new();
        local.block_on(rt, f)
    })
}
