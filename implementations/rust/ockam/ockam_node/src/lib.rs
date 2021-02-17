//! ockam_node - Ockam Node API
#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

mod context;
mod error;
mod executor;
mod mailbox;
mod messages;
mod relay;

pub use context::*;
pub use executor::*;
pub use mailbox::*;
pub use messages::*;

pub fn node() -> (Context, Executor) {
    let executor = Executor::new();
    let context = executor.new_context("app");
    (context, executor)
}
