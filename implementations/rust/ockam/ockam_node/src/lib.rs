//! ockam_node - Ockam Node API
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

pub use context::*;
pub use error::*;
pub use executor::*;
pub use node::*;
pub use worker::*;

mod context;
mod error;
mod executor;
mod node;
mod worker;

/// A unique identifier for entities in the Ockam Node.
pub type Address = String;

/// Top level [`Context`] and [`NodeExecutor`] for async main initialization.
pub fn node() -> (Context, NodeExecutor) {
    let executor = NodeExecutor::new();
    let context = executor.new_worker_context("node");
    (context, executor)
}
