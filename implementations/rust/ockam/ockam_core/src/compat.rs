//! A facade around the various collections and primitives needed to
//! support `std`, `no_std + alloc` or `no_std` targets.
//!
//! When importing from the standard library:
//!
//!   1. always prefer `core::<mod>` over `std::<mod>` where it's
//!      available. (e.g. `std::fmt::Result` -> `core::fmt::Result`)
//!   2. use `ockam_core::compat::<mod>` equivalents where
//!      possible. (e.g. `std::sync::Arc` -> `ockam_core::compat::sync::Arc`)
//!   3. if you need to add new items to compat, follow the originating
//!      namespace. (e.g. `compat::vec::Vec` and not `compat::Vec`)

/// Provides `std::borrow` for `alloc` targets.
#[cfg(feature = "alloc")]
pub use alloc::borrow;

#[doc(hidden)]
pub use futures_util::{join, try_join};

/// Provides `std::boxed` for `alloc` targets.
pub mod boxed {
    #[cfg(feature = "alloc")]
    pub use alloc::boxed::Box;
}

/// Provides `std::collections` and alternate `hashbrown` map and set
/// implementations.
pub mod collections {
    #[cfg(feature = "alloc")]
    pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

    pub use hashbrown::{HashMap, HashSet};
}

/// Provides a `std::error::Error` trait.
pub mod error {
    #[cfg(not(feature = "std"))]
    /// A `no_std` compatible definition of the `std::error::Error` trait.
    pub trait Error: core::fmt::Debug + core::fmt::Display {
        /// The source of this error.
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            None
        }
    }
    #[cfg(feature = "std")]
    pub use std::error::Error;
}

/// Provides `std::format` for `alloc` targets.
#[cfg(feature = "alloc")]
pub use alloc::format;

/// Provides `std::io`.
#[cfg(not(feature = "std"))]
pub use core2::io;
#[cfg(feature = "std")]
pub use std::io;

/// Provides `std::net`.
#[cfg(feature = "std")]
pub use std::net;

/// Provides a `println!` wrapper around `tracing::info!` for `no_std` targets
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub mod println {
    #[macro_export]
    /// Implementation of println for `no_std` by wrapping the `tracing::info!` macro.
    macro_rules! println {
        ($($arg:tt)*) => {{
            tracing::info!($($arg)*);
        }};
    }
}

/// Provides `rand`.
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
    /// A placeholder implementation of the `rand::rngs` generators module.
    ///
    /// WARNING: This implementation does NOT generate true random
    /// values, please do not try to use it in production.
    pub mod rngs {
        pub use super::not_random::OsRng;
    }

    /// Generates a random String of length 16.
    #[cfg(feature = "std")]
    pub fn random_string() -> String {
        use rand::distributions::{Alphanumeric, DistString};
        Alphanumeric.sample_string(&mut thread_rng(), 16)
    }

    /// Placeholders for various features from 'rand' that are not
    /// supported on no_std targets.
    ///
    /// WARNING: This implementation does NOT generate true random
    /// values, please do not try to use any of these in production.
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

        /// An implementation of `rand::thread_rng()` not intended for
        /// production use.
        ///
        /// WARNING: This implementation is neither random nor
        /// thread-local.
        #[allow(unsafe_code)]
        pub fn thread_rng() -> FakeRng {
            use rand::SeedableRng;
            static mut RNG: Option<rand_pcg::Lcg64Xsh32> = None;
            unsafe {
                if RNG.is_none() {
                    RNG = Some(rand_pcg::Pcg32::seed_from_u64(1234));
                }
            }
            let lcg = unsafe { rand_pcg::Pcg32::seed_from_u64(RNG.as_mut().unwrap().gen()) };

            FakeRng(lcg)
        }

        /// An implementation of `rand::random()` not intended for
        /// production use.
        pub fn random<T>() -> T
        where
            rand::distributions::Standard: rand::prelude::Distribution<T>,
        {
            let mut rng = thread_rng();
            rng.gen()
        }

        /// `rand::OsRng`
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

/// Provides `std::string`.
pub mod string {
    #[cfg(feature = "alloc")]
    pub use alloc::string::{String, ToString};
    #[cfg(not(feature = "alloc"))]
    use heapless::String as ByteString;
}

/// Provides `std::str`.
pub mod str {
    #[cfg(feature = "alloc")]
    pub use alloc::str::from_utf8;
    #[cfg(feature = "alloc")]
    pub use alloc::str::FromStr;
}

/// Provides `std::sync` for `no_std` targets.
#[cfg(not(feature = "std"))]
pub mod sync {
    use core::convert::Infallible;

    pub use alloc::sync::Arc;

    /// Wrap `spin::RwLock` as it does not return LockResult<Guard> like `std::sync::Mutex`.
    #[derive(Debug)]
    pub struct RwLock<T>(spin::RwLock<T>);
    impl<T> RwLock<T> {
        /// Creates a new spinlock wrapping the supplied data.
        pub fn new(value: T) -> Self {
            RwLock(spin::RwLock::new(value))
        }
        /// Locks this rwlock with shared read access, blocking the current thread
        /// until it can be acquired.
        pub fn read(&self) -> Result<spin::RwLockReadGuard<'_, T>, Infallible> {
            Ok(self.0.read())
        }
        /// Lock this rwlock with exclusive write access, blocking the current
        /// thread until it can be acquired.
        pub fn write(&self) -> Result<spin::RwLockWriteGuard<'_, T>, Infallible> {
            Ok(self.0.write())
        }
    }
    impl<T: Default> Default for RwLock<T> {
        fn default() -> Self {
            Self::new(Default::default())
        }
    }
    impl<T> core::ops::Deref for RwLock<T> {
        type Target = spin::RwLock<T>;
        fn deref(&self) -> &spin::RwLock<T> {
            &self.0
        }
    }
    impl<T> core::ops::DerefMut for RwLock<T> {
        fn deref_mut(&mut self) -> &mut spin::RwLock<T> {
            &mut self.0
        }
    }

    /// Wrap `spin::Mutex.lock()` as it does not return LockResult<Guard> like `std::sync::Mutex`.
    pub struct Mutex<T>(spin::Mutex<T>);
    impl<T> Mutex<T> {
        /// Creates a new mutex in an unlocked state ready for use.
        pub const fn new(value: T) -> Self {
            Mutex(spin::Mutex::new(value))
        }
        /// Acquires a mutex, blocking the current thread until it is able to do so.
        pub fn lock(&self) -> Result<spin::MutexGuard<'_, T>, Infallible> {
            Ok(self.0.lock())
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
/// Provides `std::sync` for `std` targets.
#[cfg(feature = "std")]
pub mod sync {
    pub use std::sync::Arc;
    pub use std::sync::{Mutex, RwLock};
}

/// Provides `std::task` for `no_std` targets.
#[cfg(not(feature = "std"))]
pub mod task {
    // Include both `alloc::task::*` and `core::task::*` for a better
    // approximation of `std::task::*` (which contains both).
    #[cfg(feature = "alloc")]
    pub use alloc::task::*;
    pub use core::task::*;
}

/// Provides `std::task` for `std` targets.
#[cfg(feature = "std")]
pub use std::task;

/// Provides `std::vec`.
pub mod vec {
    #[cfg(feature = "alloc")]
    pub use alloc::vec;
    #[cfg(feature = "alloc")]
    pub use alloc::vec::*;
    #[cfg(not(feature = "alloc"))]
    pub type Vec<T> = heapless::Vec<T, 64>;
}

/// Provides `std::time` for `std` targets.
#[cfg(feature = "std")]
pub mod time {
    pub use std::time::*;
}

/// Provides `std::time` for no_std targets
#[cfg(not(feature = "std"))]
pub mod time {
    pub use core::time::Duration;
}

/// Provides `core::fmt`
pub mod fmt {
    #[cfg(feature = "alloc")]
    pub use alloc::fmt::*;
    #[cfg(not(feature = "alloc"))]
    pub use core::fmt::*;
}

/// Provides `future::poll_once`
pub mod future {
    use crate::{
        errcode::{Kind, Origin},
        Error, Result,
    };
    use futures_util::future::{Future, FutureExt};

    /// Polls a future just once and returns the Result
    ///
    /// This is only used for some tests and it is hoped that we can
    /// remove it if, at some point, this makes it into `core::future`
    pub fn poll_once<'a, F, T>(future: F) -> Result<T>
    where
        F: Future<Output = Result<T>> + Send + 'a,
    {
        use core::task::{Context, Poll};
        use core::task::{RawWaker, RawWakerVTable, Waker};

        fn dummy_raw_waker() -> RawWaker {
            fn no_op(_: *const ()) {}
            fn clone(_: *const ()) -> RawWaker {
                dummy_raw_waker()
            }
            let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
            RawWaker::new(core::ptr::null(), vtable)
        }

        fn dummy_waker() -> Waker {
            // The RawWaker's vtable only contains safe no-op
            // functions which do not refer to the data field.
            #[allow(unsafe_code)]
            unsafe {
                Waker::from_raw(dummy_raw_waker())
            }
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        let result = future.boxed().poll_unpin(&mut context);
        assert!(
            result.is_ready(),
            "poll_once() only accepts futures that resolve after being polled once"
        );
        match result {
            Poll::Ready(value) => value,
            Poll::Pending => Err(Error::new_without_cause(Origin::Core, Kind::Invalid)),
        }
    }
}
