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

mod parser;
mod context;
mod error;
mod executor;
mod mailbox;
mod messages;
mod node;
mod relay;
mod router;

pub use context::*;
pub use executor::*;
pub use mailbox::*;
pub use messages::*;

pub use node::start_node;
