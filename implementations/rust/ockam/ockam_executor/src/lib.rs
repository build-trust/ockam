//! Async executor for the Ockam library.
//!
//! This crate provides an implementation of an async executor for
//! `no_std` environments and is intended for use by crates that provide
//! features and add-ons to the main Ockam library.
//!
//! The ockam_node crate re-exports types defined in this crate when the 'std'
//! feature is not enabled.
#![warn(
    //missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications
)]
#![allow(clippy::new_ret_no_self)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
#[macro_use]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
extern crate tracing;

pub mod channel;
pub mod executor;
pub mod runtime;
pub mod time;

pub mod tokio {
    pub use crate::runtime;
    pub mod sync {
        pub mod mpsc {
            pub use crate::channel::*;
        }
    }
    pub mod task {
        pub use crate::runtime;
        pub use runtime::yield_now;
        pub use runtime::JoinHandle;
    }
    pub use crate::time;
}
