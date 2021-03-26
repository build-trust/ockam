//! ockam_node - Ockam Node API
#![deny(
    // missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    // warnings
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
mod runner_relay;

pub use context::*;
pub use executor::*;
pub use mailbox::*;
pub use messages::*;

pub use node::start_node;
use tokio::runtime::Runtime;
use tokio::macros::support::Future;
use tokio::task;

pub fn block_future<F>(rt: &Runtime, f: F) -> <F as Future>::Output
    where
        F: Future + Send,
        F::Output: Send,
{
    task::block_in_place(move || {
        let local = task::LocalSet::new();
        local.block_on(&rt, f)
    })
}
