//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

#[cfg(all(feature = "std", feature = "alloc"))]
compile_error!(r#"Cannot compile both features "std" and "alloc""#);

#[cfg(all(feature = "no_std", not(feature = "alloc")))]
compile_error!(r#"The "no_std" feature currently requires the "alloc" feature"#);

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(feature = "no_std")]
#[macro_use]
extern crate core;

pub extern crate hashbrown;

#[allow(unused_imports)]
#[macro_use]
pub extern crate hex;

#[allow(unused_imports)]
#[macro_use]
pub extern crate async_trait;
pub use async_trait::async_trait as worker;

mod error;
mod message;
#[cfg(feature = "no_std")]
mod no_std_error;
mod routing;
mod worker;

pub use error::*;
pub use message::*;
pub use routing::*;
pub use worker::*;

#[cfg(feature = "std")]
pub use std::println;
#[cfg(all(feature = "no_std", feature = "alloc"))]
/// println! macro for no_std+alloc platforms
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        // TODO replace with defmt or similiar
        //cortex_m_semihosting::hprintln!($($arg)*).unwrap();
    }};
}

/// A facade around the various collections and primitives needed
/// when linking "std", "no_std + alloc" or "no_std" targets.
pub mod compat {
    #[cfg(all(feature = "no_std", feature = "alloc"))]
    pub use alloc::borrow;
    /// std::borrow
    #[cfg(feature = "std")]
    pub use std::borrow;

    /// std::boxed
    pub mod boxed {
        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use alloc::boxed::Box;
        #[cfg(feature = "std")]
        pub use std::boxed::Box;
    }

    /// std::collections
    pub mod collections {
        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};
        #[cfg(feature = "std")]
        pub use std::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

        pub use hashbrown::{HashMap, HashSet};
    }

    /// std::error::Error trait
    pub mod error {
        #[cfg(feature = "no_std")]
        pub use crate::no_std_error::Error;
        #[cfg(feature = "std")]
        pub use std::error::Error;
    }

    #[cfg(all(feature = "no_std", feature = "alloc"))]
    pub use alloc::format;
    /// std::format
    #[cfg(feature = "std")]
    pub use std::format;

    #[cfg(feature = "no_std")]
    pub use core2::io;
    /// std::io
    #[cfg(feature = "std")]
    pub use std::io;

    /// std::net
    #[cfg(feature = "std")]
    pub use std::net;

    /// rand
    pub mod rand {
        pub use rand::distributions;
        pub use rand::prelude;
        pub use rand::CryptoRng;
        pub use rand::Error;
        pub use rand::Rng;
        pub use rand::RngCore;

        #[cfg(feature = "no_std")]
        pub use not_random::thread_rng;
        #[cfg(feature = "std")]
        pub use rand::thread_rng;

        #[cfg(feature = "no_std")]
        pub use not_random::random;
        #[cfg(feature = "std")]
        pub use rand::random;

        #[cfg(feature = "std")]
        pub use rand::rngs;
        #[cfg(feature = "no_std")]
        /// rngs
        pub mod rngs {
            pub use super::not_random::OsRng;
        }

        #[cfg(feature = "no_std")]
        /// Placeholders for various features from 'rand' that are not
        /// supported on no_std targets.
        ///
        /// WARNING: Thess implementations are NOT random, please do not
        /// try to use these in production!
        mod not_random {
            use super::*;

            /// rand::thread_rng()
            /// WARNING: This implementation is neither random nor thread-local.
            pub fn thread_rng() -> rand_pcg::Lcg64Xsh32 {
                use rand::SeedableRng;
                rand_pcg::Pcg32::seed_from_u64(1234)
            }

            /// rand::random()
            pub fn random<T>() -> T
            where
                rand::distributions::Standard: rand::prelude::Distribution<T>,
            {
                let mut rng = thread_rng();
                rng.gen()
            }

            /// rand::OsRng
            pub struct OsRng;

            impl CryptoRng for OsRng {}

            impl RngCore for OsRng {
                fn next_u32(&mut self) -> u32 {
                    let mut rng = thread_rng();
                    rng.gen()
                }

                fn next_u64(&mut self) -> u64 {
                    let mut rng = thread_rng();
                    rng.gen()
                }

                fn fill_bytes(&mut self, dest: &mut [u8]) {
                    if let Err(e) = self.try_fill_bytes(dest) {
                        panic!("Error: {}", e);
                    }
                }

                fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
                    let mut rng = thread_rng();
                    rng.try_fill(dest)
                }
            }
        }
    }

    /// std::string
    pub mod string {
        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use alloc::string::{String, ToString};
        #[cfg(all(feature = "no_std", not(feature = "alloc")))]
        use heapless::String as ByteString;
        #[cfg(feature = "std")]
        pub use std::string::{String, ToString};
    }

    /// std::sync
    pub mod sync {
        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use alloc::sync::Arc;
        #[cfg(feature = "std")]
        pub use std::sync::Arc;

        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use spin::Mutex;
        #[cfg(feature = "std")]
        pub use std::sync::Mutex;
    }

    /// std::vec
    pub mod vec {
        #[cfg(all(feature = "no_std", feature = "alloc"))]
        pub use alloc::vec::Vec;
        #[cfg(feature = "std")]
        pub use std::vec::Vec;
        #[cfg(all(feature = "no_std", not(feature = "alloc")))]
        pub type Vec<T> = heapless::Vec<T, 64>;
    }
}
