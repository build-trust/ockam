//! Async executor for the Ockam library.
//!
//! This crate provides an implementation of an async executor for
//! `no_std` environments and is intended for use by crates that provide
//! features and add-ons to the main Ockam library.
//!
//! The ockam_node crate re-exports types defined in this crate when the 'std'
//! feature is not enabled.
#![allow(
    //missing_docs,
    //trivial_casts,
    trivial_numeric_casts,
    //unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(unused_imports)]

#[cfg(feature = "std")]
#[allow(unused_imports)]
#[macro_use]
extern crate core;

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod channel;
pub mod executor;
pub mod oneshot;
pub mod runtime;
pub mod time;

pub mod tokio {
    pub use crate::runtime;
    pub mod sync {
        pub mod mpsc {
            pub use crate::channel::*;
        }
        pub use crate::oneshot;
    }
    pub mod task {
        pub use crate::runtime;
        pub use runtime::yield_now;
        pub use runtime::JoinHandle;
    }
    pub use crate::time;
}

// simple logging

#[cfg(not(feature = "std"))]
pub use ockam_core::println;

#[cfg(not(feature = "std"))]
pub mod logging_no_std {
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
