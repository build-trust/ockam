//! A facade around the various collections and primitives needed
//! when using "std", "no_std + alloc" or "no_std" targets.
//!
//! When importing from the standard library:
//!
//!   1. always prefer core::<mod> over std::<mod> where it's
//!      available. (e.g. std::fmt::Result -> core::fmt::Result)
//!   2. use ockam_core::compat::<mod> equivalents where
//!      possible. (e.g. std::sync::Arc -> ockam_core::compat::sync::Arc)
//!   3. if you need to add new items to compat, follow the originating
//!      namespace. (e.g. compat::vec::Vec and not compat::Vec)
#![allow(missing_docs)]

/// std::borrow
#[cfg(feature = "alloc")]
pub use alloc::borrow;

#[doc(hidden)]
pub use futures_util::try_join;

/// std::boxed
pub mod boxed {
    #[cfg(feature = "alloc")]
    pub use alloc::boxed::Box;
}

/// std::collections
pub mod collections {
    #[cfg(feature = "alloc")]
    pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

    pub use hashbrown::{HashMap, HashSet};
}

/// std::error::Error trait
pub mod error {
    #[cfg(not(feature = "std"))]
    pub trait Error: core::fmt::Debug + core::fmt::Display {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            None
        }
    }
    #[cfg(feature = "std")]
    pub use std::error::Error;
}

/// std::format
#[cfg(feature = "alloc")]
pub use alloc::format;

/// std::io
#[cfg(not(feature = "std"))]
pub use core2::io;
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

    #[cfg(not(feature = "std"))]
    pub use not_random::thread_rng;
    #[cfg(feature = "std")]
    pub use rand::thread_rng;

    #[cfg(not(feature = "std"))]
    pub use not_random::random;
    #[cfg(feature = "std")]
    pub use rand::random;

    /// rngs
    #[cfg(feature = "std")]
    pub use rand::rngs;
    #[cfg(not(feature = "std"))]
    pub mod rngs {
        pub use super::not_random::OsRng;
    }

    /// Placeholders for various features from 'rand' that are not
    /// supported on no_std targets.
    ///
    /// WARNING: These implementations are NOT random, please do not
    /// try to use these in production!
    #[cfg(not(feature = "std"))]
    mod not_random {
        use super::*;

        #[derive(Clone)]
        pub struct FakeRng(rand_pcg::Lcg64Xsh32);

        impl CryptoRng for FakeRng {}

        impl RngCore for FakeRng {
            fn next_u32(&mut self) -> u32 {
                self.0.gen()
            }

            fn next_u64(&mut self) -> u64 {
                self.0.gen()
            }

            fn fill_bytes(&mut self, dest: &mut [u8]) {
                if let Err(e) = self.0.try_fill_bytes(dest) {
                    panic!("Error: {}", e);
                }
            }

            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
                self.0.try_fill(dest)
            }
        }

        /// rand::thread_rng()
        /// WARNING: This implementation is neither random nor thread-local.
        #[allow(unsafe_code)]
        pub fn thread_rng() -> FakeRng {
            use rand::SeedableRng;
            // TODO safety
            static mut RNG: Option<rand_pcg::Lcg64Xsh32> = None;
            unsafe {
                if RNG.is_none() {
                    RNG = Some(rand_pcg::Pcg32::seed_from_u64(1234));
                }
            }
            let lcg = unsafe { rand_pcg::Pcg32::seed_from_u64(RNG.as_mut().unwrap().gen()) };

            FakeRng(lcg)
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
    #[cfg(feature = "alloc")]
    pub use alloc::string::{String, ToString};
    #[cfg(not(feature = "alloc"))]
    use heapless::String as ByteString;
}

/// std::sync
#[cfg(not(feature = "std"))]
pub mod sync {
    pub use alloc::sync::Arc;
    pub use spin::RwLock;

    /// spin::Mutex.lock() does not return Option<T>
    pub struct Mutex<T>(spin::Mutex<T>);
    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Mutex(spin::Mutex::new(value))
        }
        pub fn lock(&self) -> Option<spin::MutexGuard<'_, T>> {
            Some(self.0.lock())
        }
    }
    impl<T> core::ops::Deref for Mutex<T> {
        type Target = spin::Mutex<T>;
        fn deref(&self) -> &spin::Mutex<T> {
            &self.0
        }
    }
    impl<T> core::ops::DerefMut for Mutex<T> {
        fn deref_mut(&mut self) -> &mut spin::Mutex<T> {
            &mut self.0
        }
    }
}
#[cfg(feature = "std")]
pub mod sync {
    pub use std::sync::Arc;
    pub use std::sync::{Mutex, RwLock};
}

/// std::task
#[cfg(feature = "std")]
pub use std::task;

#[cfg(not(feature = "std"))]
pub mod task {
    // Include both `alloc::task::*` and `core::task::*` for a better
    // approximation of `std::task::*` (which contains both).
    #[cfg(feature = "alloc")]
    pub use alloc::task::*;
    pub use core::task::*;
}

/// std::vec
pub mod vec {
    #[cfg(feature = "alloc")]
    pub use alloc::vec::*;
    #[cfg(not(feature = "alloc"))]
    pub type Vec<T> = heapless::Vec<T, 64>;
}
