//! Core types of the Ockam library.
//!
//! This crate contains the core types of the Ockam library and is intended
//! for use by other crates that provide features and add-ons to the main
//! Ockam library.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![no_std]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

#[cfg(all(feature = "no_std", feature = "alloc"))]
compile_error!(r#"Cannot compile both features "alloc" and "no_std""#);

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
#[macro_use]
extern crate std;

pub extern crate hashbrown;

#[allow(unused_imports)]
#[macro_use]
pub extern crate hex;

#[allow(unused_imports)]
#[macro_use]
pub extern crate async_trait;

mod error;
mod message;
mod routing;
mod worker;

pub use error::*;
pub use message::*;
pub use routing::*;
pub use worker::*;

/// A facade around the various collections and primitives needed
/// when using no alloc, alloc only, or std modes
pub mod lib {
    mod core {
        #[cfg(not(feature = "std"))]
        pub use core::*;
        #[cfg(feature = "std")]
        pub use std::*;
    }

    pub use self::core::cell::{Cell, RefCell};
    pub use self::core::clone::{self, Clone};
    pub use self::core::convert::{self, From, Into};
    pub use self::core::default::{self, Default};
    pub use self::core::fmt::{self, Debug, Display};
    pub use self::core::marker::{self, PhantomData};
    pub use self::core::num::Wrapping;
    pub use self::core::ops::{Deref, DerefMut, Range};
    pub use self::core::option::{self, Option};
    pub use self::core::result::{self, Result};
    pub use self::core::{cmp, iter, mem, num, slice, str};
    pub use self::core::{f32, f64};
    pub use self::core::{i16, i32, i64, i8, isize};
    pub use self::core::{u16, u32, u64, u8, usize};

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::borrow::{Cow, ToOwned};
    #[cfg(feature = "std")]
    pub use std::borrow::{Cow, ToOwned};

    #[cfg(all(not(feature = "alloc"), feature = "no_std"))]
    pub use heapless::consts::*;

    #[cfg(all(not(feature = "alloc"), feature = "no_std"))]
    use heapless::String as ByteString;
    #[cfg(all(not(feature = "alloc"), feature = "no_std"))]
    pub type String = ByteString<U128>;
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::string::{String, ToString};
    #[cfg(feature = "std")]
    pub use std::string::{String, ToString};

    #[cfg(all(not(feature = "alloc"), feature = "no_std"))]
    use heapless::Vec as InternalVec;
    #[cfg(all(not(feature = "alloc"), feature = "no_std"))]
    pub type Vec<T> = InternalVec<T, U64>;
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::vec::Vec;
    #[cfg(feature = "std")]
    pub use std::vec::Vec;

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::boxed::Box;
    #[cfg(feature = "std")]
    pub use std::boxed::Box;

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};
    #[cfg(feature = "std")]
    pub use std::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque};

    #[cfg(feature = "std")]
    pub use std::{error, net};

    pub use hashbrown::{HashMap, HashSet};
}
