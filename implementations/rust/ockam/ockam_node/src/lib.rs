//! ockam_node - Ockam Node API

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    missing_docs,
    dead_code,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!(r#"Cannot compile both features "std" and "alloc""#);

#[cfg(all(feature = "no_std", not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "no_std")]
#[macro_use]
extern crate core;

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
#[macro_use]
extern crate tracing;

#[cfg(feature = "no_std")]
pub use ockam_node_no_std::tokio;
#[cfg(feature = "std")]
pub use tokio;

#[cfg(feature = "no_std")]
pub use ockam_node_no_std::interrupt; // TODO replace with ockam_core::compat::sync::*

#[cfg(feature = "no_std")]
/// TODO replace with defmt
#[macro_use]
mod logging_no_std {
    /// info!
    #[macro_export]
    macro_rules! info {
        ($($arg:tt)*) => (
            ockam_core::println!($($arg)*);
        )
    }
    /// trace!
    #[macro_export]
    macro_rules! trace {
        ($($arg:tt)*) => (
            ockam_core::println!($($arg)*);
        )
    }
    /// error!
    #[macro_export]
    macro_rules! error {
        ($($arg:tt)*) => (
            ockam_core::println!($($arg)*);
        )
    }
    /// debug!
    #[macro_export]
    macro_rules! debug {
        ($($arg:tt)*) => (
            ockam_core::println!($($arg)*);
        )
    }
}

mod address_record;
mod context;
mod error;
mod executor;
mod mailbox;
mod messages;
mod node;
mod parser;
mod relay;
mod router;
mod tests;

pub(crate) use address_record::*;
pub use context::*;
pub use executor::*;
pub use mailbox::*;
pub use messages::*;

pub use node::{start_node, NullWorker};

#[cfg(feature = "std")]
use crate::tokio::{runtime::Runtime, task};
#[cfg(feature = "std")]
use core::future::Future;

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

#[doc(hidden)]
pub fn spawn<F: 'static>(f: F)
where
    F: Future + Send,
    F::Output: Send,
{
    task::spawn(f);
}

#[cfg(feature = "no_std")]
pub use crate::tokio::{block_future, execute};
